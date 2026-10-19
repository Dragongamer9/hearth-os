//! Game Mode: talking to gamescope through X11 root-window properties, the way Steam does (port of
//! hosts/linux/src/gamescope.js), plus the frame-rate limit, the performance overlay (mangoapp) and screenshots.
//!   GAMESCOPE_FOCUSABLE_WINDOWS     what can be shown: (window, app id, pid) triples, set by gamescope
//!   GAMESCOPE_FOCUSED_WINDOW        what's in front now, set by gamescope
//!   GAMESCOPECTRL_BASELAYER_WINDOW  which window to put in front, set by us (removed = gamescope decides)
//!   GAMESCOPECTRL_BASELAYER_APPID   same by Steam app id (games Steam starts carry their app id)
//!   GAMESCOPE_FPS_LIMIT             frame-rate limit, 0 = none
//! Needs xprop; everything here is best effort.

use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub fn in_session() -> bool { std::env::var("NIMBUS_SESSION").map(|v| v == "1").unwrap_or(false) }
pub fn available() -> bool { crate::system::has("xprop") && std::env::var_os("DISPLAY").is_some() }
/// Running inside gamescope (Game Mode).
pub fn inside() -> bool { std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some() && available() }

fn xprop(args: &[&str]) -> String { crate::system::run_timeout("xprop", args, Duration::from_secs(3)).unwrap_or_default() }

/// Numbers in a CARDINAL/WINDOW property of the root window (or of a window).
pub fn parse_cardinals(out: &str) -> Vec<u64> {
    let Some(i) = out.find('=') else { return vec![] };
    out[i + 1..].lines().next().unwrap_or("").split(',').filter_map(|s| {
        let s = s.trim();
        if let Some(h) = s.strip_prefix("0x") { u64::from_str_radix(h, 16).ok() } else { s.parse().ok() }
    }).collect()
}
pub fn cardinals(name: &str, win: Option<u64>) -> Vec<u64> {
    let id = win.map(|w| w.to_string());
    parse_cardinals(&match &id { Some(w) => xprop(&["-id", w, name]), None => xprop(&["-root", name]) })
}
fn str_prop(win: u64, name: &str) -> String {
    let out = xprop(&["-id", &win.to_string(), name]);
    out.split_once("= \"").map(|(_, r)| r.trim_end().trim_end_matches('"').to_string()).unwrap_or_default()
}
pub fn set_root(name: &str, v: u64) -> Result<(), String> { crate::system::run_timeout("xprop", &["-root", "-f", name, "32c", "-set", name, &v.to_string()], Duration::from_secs(3)).map(|_| ()) }
pub fn remove_root(name: &str) { let _ = crate::system::run_timeout("xprop", &["-root", "-remove", name], Duration::from_secs(3)); }
pub fn set_window(win: u64, name: &str, v: u64) -> Result<(), String> {
    crate::system::run_timeout("xprop", &["-id", &win.to_string(), "-f", name, "32c", "-set", name, &v.to_string()], Duration::from_secs(3)).map(|_| ())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Win { pub win: u64, pub appid: u64, pub pid: u64, pub title: String, pub w: u32, pub h: u32 }

pub fn focusable_windows() -> Vec<Win> {
    cardinals("GAMESCOPE_FOCUSABLE_WINDOWS", None).chunks(3).filter(|c| c.len() == 3)
        .map(|c| Win { win: c[0], appid: c[1], pid: c[2], title: String::new(), w: 0, h: 0 }).collect()
}
pub fn focused() -> Option<u64> { cardinals("GAMESCOPE_FOCUSED_WINDOW", None).first().copied() }
fn proc_name(pid: u64) -> String { std::fs::read_to_string(format!("/proc/{pid}/comm")).map(|s| s.trim().to_string()).unwrap_or_default() }

/// "xwininfo -root -children" lines → top-level windows with a name: (win, title, w, h)
pub fn parse_children(text: &str) -> Vec<(u64, String, u32, u32)> {
    let re = regex::Regex::new(r#"(?i)^\s*(0x[0-9a-f]+)\s+"(.*)":\s.*?\s(\d+)x(\d+)[+-]"#).unwrap();
    text.lines().filter_map(|l| {
        let c = re.captures(l)?;
        let title = c[2].to_string();
        if title.is_empty() || title == "has no name" { return None; }
        Some((u64::from_str_radix(&c[1][2..], 16).ok()?, title, c[3].parse().ok()?, c[4].parse().ok()?))
    }).collect()
}
/// Top-level windows you can see, with their process.
pub fn mapped_windows() -> Vec<Win> {
    if !crate::system::has("xwininfo") { return vec![]; }
    let kids = parse_children(&crate::system::run_timeout("xwininfo", &["-root", "-children"], Duration::from_secs(3)).unwrap_or_default());
    let mut out = Vec::new();
    for (win, title, w, h) in kids.into_iter().filter(|k| k.2 >= 200 && k.3 >= 150).take(40) {
        let st = crate::system::run_timeout("xwininfo", &["-id", &win.to_string()], Duration::from_secs(2)).unwrap_or_default();
        if !st.contains("IsViewable") { continue; }
        let pid = cardinals("_NET_WM_PID", Some(win)).first().copied().unwrap_or(0);
        let appid = cardinals("STEAM_GAME", Some(win)).first().copied().unwrap_or(0);
        out.push(Win { win, appid, pid, title, w, h });
    }
    out
}
/// A window by its exact title (Hearth's own windows: the main one and the overlay).
pub fn window_named(title: &str) -> Option<u64> {
    let out = crate::system::run_timeout("xwininfo", &["-name", title], Duration::from_secs(2)).ok()?;
    let i = out.find("Window id: 0x")?;
    let hex: String = out[i + 13..].chars().take_while(|c| c.is_ascii_hexdigit()).collect();
    u64::from_str_radix(&hex, 16).ok()
}

/// Open apps for the overlay: [{id, pid, appid, title, app, focused}], one per process.
pub fn apps(exclude: &[u64]) -> Vec<Value> {
    let f = focused();
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for w in focusable_windows().into_iter().chain(mapped_windows()) {
        let key = if w.pid > 0 { format!("p{}", w.pid) } else { format!("w{}", w.win) };
        if !seen.insert(key) || exclude.contains(&w.pid) || exclude.contains(&w.win) { continue; }
        let mut title = w.title.clone();
        if title.is_empty() { title = str_prop(w.win, "_NET_WM_NAME"); }
        if title.is_empty() { title = str_prop(w.win, "WM_NAME"); }
        let app = proc_name(w.pid);
        if title.is_empty() { title = if app.is_empty() { "App".into() } else { app.clone() }; }
        out.push(json!({"id": w.win, "pid": w.pid, "appid": w.appid, "title": title, "app": app, "focused": f == Some(w.win)}));
    }
    out
}

/// Puts one window in front. A Steam app id set earlier would win over it, so that goes first.
pub fn focus_window(win: u64) {
    remove_root("GAMESCOPECTRL_BASELAYER_APPID");
    let _ = set_root("GAMESCOPECTRL_BASELAYER_WINDOW", win);
    restore(win);
}
/// A game that minimized itself when something else came in front draws nothing (a black screen with sound):
/// ask for it to be shown again.
pub fn restore(win: u64) {
    if win == 0 || !crate::system::has("xdotool") { return; }
    let w = win.to_string();
    let _ = crate::system::run_timeout("xdotool", &["windowmap", &w], Duration::from_secs(2));
    let _ = crate::system::run_timeout("xdotool", &["windowactivate", &w], Duration::from_secs(2));
}
/// By Steam app id; a window pinned earlier (maybe a game put aside, frozen) would win over it, so it goes.
pub fn focus_app_id(appid: u64) { remove_root("GAMESCOPECTRL_BASELAYER_WINDOW"); let _ = set_root("GAMESCOPECTRL_BASELAYER_APPID", appid); }
pub fn clear_focus() { remove_root("GAMESCOPECTRL_BASELAYER_WINDOW"); remove_root("GAMESCOPECTRL_BASELAYER_APPID"); }

/// Brings a game to the front once its window exists (games take a while to open one): matching its processes
/// or its Steam app id, else the window it last had (`known`). Windows of other games (`not_wins`, and the
/// processes in `exclude`) are never picked. `still` says whether this is still wanted. Its window when found.
pub fn focus_game(pids: &dyn Fn() -> Vec<u32>, steam_app_id: u64, exclude: &[u64], not_wins: &[u64], known: u64, still: &dyn Fn() -> bool) -> Option<u64> {
    if !available() { return None; }
    if steam_app_id > 0 { focus_app_id(steam_app_id); }
    for _ in 0..20 {
        if !still() { return None; }
        let want: Vec<u64> = pids().into_iter().map(u64::from).collect();
        let wins: Vec<Win> = focusable_windows().into_iter().filter(|w| !exclude.contains(&w.pid) && !not_wins.contains(&w.win)).collect();
        let mut hit = wins.iter().find(|w| want.contains(&w.pid)).or_else(|| if steam_app_id > 0 { wins.iter().find(|w| w.appid == steam_app_id) } else { None });
        // The window this game had before it was put aside (its processes weren't matched: a launcher's helper).
        if hit.is_none() && known != 0 { hit = wins.iter().find(|w| w.win == known); }
        // Its processes not matched (some launchers hand the window to a helper): if only one window is left
        // that isn't Steam's, Hearth's or a game put aside, that's it.
        if hit.is_none() && steam_app_id == 0 {
            let rest: Vec<&Win> = wins.iter().filter(|w| w.pid > 0 && !matches!(proc_name(w.pid).as_str(), "steam" | "steamwebhelper")).collect();
            if rest.len() == 1 { hit = rest.first().copied(); }
        }
        if let Some(h) = hit { if !still() { return None; } focus_window(h.win); crate::bridge::log(&format!("gamescope: game window {} in front", h.win)); return Some(h.win); }
        std::thread::sleep(Duration::from_millis(1500));
    }
    None
}

pub enum Ready { Window(u64), Dialog(u64, String) }

/// Waits until a game (or its launcher) has a real window up while Hearth shows its "starting" screen: a window
/// of the game's processes (or with its Steam app id), at least 640x360, seen twice in a row. A Steam window
/// that stays up a few seconds while the game shows nothing may need you (a cloud-save question): Dialog.
pub fn wait_for_window(pids: &dyn Fn() -> Vec<u32>, steam_app_id: u64, exclude: &[u64], timeout: Duration, cancelled: &dyn Fn() -> bool) -> Option<Ready> {
    if !available() { return None; }
    let start = Instant::now();
    let mut last = 0;
    let mut steam_seen: std::collections::HashMap<u64, Instant> = Default::default();
    while start.elapsed() < timeout {
        if cancelled() { return None; }
        let want: Vec<u64> = pids().into_iter().map(u64::from).collect();
        let wins: Vec<Win> = mapped_windows().into_iter().filter(|w| !exclude.contains(&w.pid)).collect();
        let game = wins.iter().find(|w| (want.contains(&w.pid) || (steam_app_id > 0 && w.appid == steam_app_id)) && w.w >= 640 && w.h >= 360);
        match game { Some(g) if g.win == last => return Some(Ready::Window(g.win)), Some(g) => last = g.win, None => last = 0 }
        for w in &wins {
            let name = proc_name(w.pid);
            if (name == "steam" || name == "steamwebhelper") && w.w >= 480 && w.h >= 240 && !want.contains(&w.pid) {
                let seen = *steam_seen.entry(w.win).or_insert_with(Instant::now);
                if game.is_none() && seen.elapsed() > Duration::from_millis(3500) && start.elapsed() > Duration::from_secs(5) { return Some(Ready::Dialog(w.win, w.title.clone())); }
            }
        }
        std::thread::sleep(Duration::from_millis(700));
    }
    None
}

/// Frame-rate limit for games (gamescope's own limiter); 0 = none.
pub fn set_fps_limit(fps: u64) -> Result<u64, String> {
    let n = fps.min(240);
    if !inside() { return Err("The frame-rate limit works in Game Mode.".into()); }
    set_root("GAMESCOPE_FPS_LIMIT", n)?;
    Ok(n)
}
pub fn can_fps_limit() -> bool { inside() }

// ------------------------------------------------------------------ performance overlay (mangoapp)
// The session starts gamescope with --mangoapp and points MANGOHUD_CONFIGFILE at a file mangoapp watches:
// writing a new layout there changes what's shown over the game at once.

pub const PERF_PRESETS: [&str; 5] = [
    "no_display",
    "legacy_layout=false\nfps_only\nfont_size=20\nbackground_alpha=0.4\nposition=top-left\nhud_no_margin",
    "legacy_layout=false\nhorizontal\nhud_compact\nfps\nframetime=0\ncpu_stats\ngpu_stats\nbattery\nfont_size=20\nbackground_alpha=0.4\nposition=top-center\ntable_columns=14",
    "legacy_layout=false\nfps\nframe_timing=1\ncpu_stats\ncpu_temp\ngpu_stats\ngpu_temp\nram\nvram\nbattery\nbattery_time\nfont_size=20\nbackground_alpha=0.5\nposition=top-left",
    "legacy_layout=false\nfps\nframe_timing=1\nframetime\ncpu_stats\ncpu_temp\ncpu_power\ncpu_mhz\ncore_load\ngpu_stats\ngpu_temp\ngpu_power\ngpu_core_clock\ngpu_mem_clock\nram\nvram\nswap\nbattery\nbattery_watt\nbattery_time\nfan\nresolution\nengine_version\nwine\ngamemode\nfont_size=18\nbackground_alpha=0.5\nposition=top-left",
];
fn perf_file() -> Option<String> { std::env::var("MANGOHUD_CONFIGFILE").ok().filter(|s| !s.is_empty()) }
pub fn perf_overlay_supported() -> bool { perf_file().map(|f| std::path::Path::new(&f).exists()).unwrap_or(false) }
pub fn set_perf_overlay(level: u64) -> Result<u64, String> {
    let f = perf_file().ok_or("The performance overlay works in Game Mode.")?;
    let n = level.min(PERF_PRESETS.len() as u64 - 1);
    let tmp = format!("{f}.tmp");
    std::fs::write(&tmp, format!("{}\n", PERF_PRESETS[n as usize])).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &f).map_err(|e| e.to_string())?;
    Ok(n)
}

// ------------------------------------------------------------------ screenshots

/// Where Hearth keeps screenshots: Pictures/Screenshots/Hearth.
pub fn screenshot_dir() -> std::path::PathBuf {
    let pics = crate::system::run("xdg-user-dir", &["PICTURES"]).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty() && s != "/")
        .unwrap_or_else(|| crate::paths::home().join("Pictures").to_string_lossy().to_string());
    std::path::Path::new(&pics).join("Screenshots/Hearth")
}

/// gamescope takes it itself, the way Steam asks for one: GAMESCOPECTRL_REQUEST_SCREENSHOT on the root window
/// (2 = the game and other real windows, without overlays), written to /tmp/gamescope.png; gamescope removes
/// the property when it's done.
fn gamescope_shot(to: &std::path::Path) -> bool {
    if !available() { return false; }
    let tmp = std::path::Path::new("/tmp/gamescope.png");
    let before = std::fs::metadata(tmp).and_then(|m| m.modified()).ok();
    if set_root("GAMESCOPECTRL_REQUEST_SCREENSHOT", 2).is_err() { return false; }
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(4) {
        std::thread::sleep(Duration::from_millis(150));
        let now = std::fs::metadata(tmp).and_then(|m| m.modified()).ok();
        let done = !xprop(&["-root", "GAMESCOPECTRL_REQUEST_SCREENSHOT"]).contains('=');
        if now.is_some() && now != before && done {
            std::thread::sleep(Duration::from_millis(100));
            return std::fs::copy(tmp, to).map(|n| n > 0).unwrap_or(false);
        }
    }
    remove_root("GAMESCOPECTRL_REQUEST_SCREENSHOT");
    false
}

/// Saves a screenshot to Pictures/Screenshots/Hearth: in Game Mode gamescope takes it, otherwise whichever
/// tool this desktop has. (The host gets Hearth's own windows out of the way first.)
pub fn screenshot() -> Result<Value, String> {
    let dir = screenshot_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join(format!("Screenshot {}.png", chrono::Local::now().format("%Y-%m-%d %H-%M-%S")));
    let f = file.to_string_lossy().to_string();
    if inside() && gamescope_shot(&file) { return Ok(json!({"file": f, "url": crate::library::art_url(&f)})); }
    let tries: [(&str, Vec<&str>); 4] = [("grim", vec![&f]), ("spectacle", vec!["-b", "-n", "-f", "-o", &f]), ("gnome-screenshot", vec!["-f", &f]), ("gamescopectl", vec!["screenshot", &f])];
    for (cmd, args) in tries.iter() {
        if !crate::system::has(cmd) { continue; }
        if crate::system::run_timeout(cmd, args, Duration::from_secs(10)).is_err() { continue; }
        if file.exists() { return Ok(json!({"file": f, "url": crate::library::art_url(&f)})); }
    }
    Err("Couldn't take a screenshot here. In Desktop Mode it needs grim or Spectacle.".into())
}

/// Screenshots, newest first: [{file, url, name, time, size}]
pub fn screenshots() -> Vec<Value> {
    let mut v: Vec<(u64, Value)> = std::fs::read_dir(screenshot_dir()).into_iter().flatten().flatten().filter_map(|e| {
        let p = e.path();
        let ext = p.extension()?.to_str()?.to_ascii_lowercase();
        if !["png", "jpg", "jpeg", "webp"].contains(&ext.as_str()) { return None; }
        let m = e.metadata().ok()?;
        let t = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis() as u64;
        let f = p.to_string_lossy().to_string();
        Some((t, json!({"file": f, "url": crate::library::art_url(&f), "name": p.file_stem()?.to_string_lossy(), "time": t, "size": m.len()})))
    }).collect();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v.into_iter().map(|x| x.1).collect()
}

/// Deletes one screenshot (only from Hearth's screenshots folder).
pub fn delete_screenshot(file: &str) -> Result<(), String> {
    let dir = screenshot_dir().canonicalize().map_err(|_| "No screenshots yet".to_string())?;
    let p = std::path::Path::new(file).canonicalize().map_err(|_| "That screenshot is already gone".to_string())?;
    if p.parent() != Some(dir.as_path()) { return Err("Only Hearth's screenshots can be deleted here".into()); }
    std::fs::remove_file(&p).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_xprop_and_xwininfo() {
        assert_eq!(parse_cardinals("GAMESCOPE_FOCUSABLE_WINDOWS(CARDINAL) = 4194306, 769, 1234, 0x600001, 0, 99"), vec![4194306, 769, 1234, 0x600001, 0, 99]);
        assert!(parse_cardinals("GAMESCOPE_FOCUSED_WINDOW:  not found.").is_empty());
        let kids = parse_children("     0x400002 \"Hearth\": (\"hearth\" \"Hearth\")  1280x800+0+0  +0+0\n     0x400010 (has no name): ()  10x10+0+0  +0+0\n");
        assert_eq!(kids, vec![(0x400002, "Hearth".to_string(), 1280, 800)]);
    }
}
