//! Game Mode: talking to gamescope through X11 root-window properties, the way Steam does (port of
//! hosts/linux/src/gamescope.js), plus the frame-rate limit, the performance overlay (mangoapp) and screenshots.
//!   GAMESCOPE_FOCUSABLE_WINDOWS     what can be shown: (window, app id, pid) triples, set by gamescope
//!   GAMESCOPE_FOCUSED_WINDOW        what's in front now, set by gamescope
//!   GAMESCOPECTRL_BASELAYER_APPID   which app ids to show, in order: the first one with a window wins (set by us)
//!   GAMESCOPECTRL_BASELAYER_WINDOW  one window to show instead, whatever its id (set by us, only for windows that
//!                                   aren't a game's: a Steam question, an app picked in Quick Access)
//!   STEAM_GAME (on a window)        its app id; gamescope takes it over whatever it worked out itself
//!   GAMESCOPE_FPS_LIMIT             frame-rate limit, 0 = none
//!
//! Like SteamOS, every game has a number (appid.rs) and Hearth shows a game by asking for its number, then Hearth's
//! own (HEARTH_ID), so gamescope picks the game's best window (the game over its launcher or splash) and never a
//! window of a game put aside, whose number isn't asked for; with no game window up, Hearth shows, never nothing.
//!
//! gamescope only follows those requests when it runs in Steam mode (-e) or its focus strategy is SteamControlled;
//! otherwise (gamescope 3.16+ started without -e, as Hearth's session does) it ignores them and shows the window
//! mapped last. Hearth switches the strategy at start (`take_focus_control`). Without -e, gamescope gives a window
//! its STEAM_GAME number when it's set, and forgets it when the window is mapped again: Hearth puts it back.
//! Needs xprop; everything here is best effort.

use crate::appid::HEARTH_ID;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::time::{Duration, Instant};

pub const BASELAYER_APPID: &str = "GAMESCOPECTRL_BASELAYER_APPID";
pub const BASELAYER_WINDOW: &str = "GAMESCOPECTRL_BASELAYER_WINDOW";

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
/// A list of cardinals on the root window (xprop takes "a,b,c"). Read back, so the log shows it if this xprop
/// only wrote the first one (then the game's number is still asked for, without Hearth's after it).
pub fn set_root_list(name: &str, vals: &[u32]) -> Result<(), String> {
    let v = vals.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",");
    crate::system::run_timeout("xprop", &["-root", "-f", name, "32c", "-set", name, &v], Duration::from_secs(3))?;
    if vals.len() > 1 {
        let back: Vec<u64> = cardinals(name, None);
        if back != vals.iter().map(|n| *n as u64).collect::<Vec<u64>>() { crate::bridge::log(&format!("gamescope: asked for {name} = {vals:?}, it holds {back:?}")); }
    }
    Ok(())
}
/// Puts a game's (or Hearth's) number on a window. Never 0: gamescope takes 0 as "no number".
pub fn tag_window(win: u64, id: u32) -> bool { win != 0 && id != 0 && set_window(win, "STEAM_GAME", id as u64).is_ok() }

static STEAM_MODE: AtomicU8 = AtomicU8::new(0);   // 0 not known yet, 1 yes, 2 no
static STEAM_MODE_AT: AtomicU64 = AtomicU64::new(0);
/// gamescope started with -e (Steam mode): only then does it publish GAMESCOPE_FOCUSED_APP. Asked again now and
/// then while the answer is no (gamescope may not have published anything yet).
pub fn steam_mode() -> bool {
    let now = crate::library::now_ms();
    match STEAM_MODE.load(Ordering::SeqCst) {
        1 => return true,
        2 if now.saturating_sub(STEAM_MODE_AT.load(Ordering::SeqCst)) < 30_000 => return false,
        _ => {}
    }
    let out = xprop(&["-root", "GAMESCOPE_FOCUSED_APP"]);
    if out.trim().is_empty() { return false; }   // xprop didn't answer: don't remember anything
    let yes = out.contains("GAMESCOPE_FOCUSED_APP(");
    STEAM_MODE.store(if yes { 1 } else { 2 }, Ordering::SeqCst);
    STEAM_MODE_AT.store(now, Ordering::SeqCst);
    yes
}

/// gamescope 3.16+ without -e (Steam mode) uses the "SingleApplication" focus strategy, which ignores
/// GAMESCOPECTRL_BASELAYER_WINDOW/APPID and always shows the window mapped last (why a game put aside could come
/// back in place of the one asked for). Steam's strategy, SteamControlled, follows them: switched on through
/// gamescopectl (backend_virtual_connector_strategy 1). What happened, for the log.
pub fn take_focus_control() -> String {
    if steam_mode() { return "gamescope runs in Steam mode (-e): it follows Hearth's app ids already".into(); }
    if !crate::system::has("gamescopectl") { return "gamescopectl isn't installed: gamescope may ignore which game Hearth asks for".into(); }
    match crate::system::run_timeout("gamescopectl", &["backend_virtual_connector_strategy", "1"], Duration::from_secs(5)) {
        Ok(_) => "gamescope's focus strategy set to SteamControlled (it follows Hearth's app ids and pinned windows)".into(),
        Err(e) => format!("couldn't set gamescope's focus strategy ({e}): it may ignore which game Hearth asks for"),
    }
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

/// What gamescope is asked for: `front`'s number first, then Hearth's, so the screen shows Hearth (never nothing)
/// while the game has no window up.
pub fn focus_list(front: Option<u32>) -> Vec<u32> {
    let mut v: Vec<u32> = front.into_iter().filter(|n| *n != 0 && *n != HEARTH_ID).collect();
    v.push(HEARTH_ID);
    v
}
/// Shows by number (see `focus_list`): any pinned window goes, so gamescope picks among the windows carrying them.
pub fn focus_ids(ids: &[u32]) {
    let mut v: Vec<u32> = Vec::new();
    for n in ids { if *n != 0 && !v.contains(n) { v.push(*n); } }
    if v.is_empty() { v.push(HEARTH_ID); }
    remove_root(BASELAYER_WINDOW);
    let _ = set_root_list(BASELAYER_APPID, &v);
}
/// The app ids asked for now.
pub fn requested_ids() -> Vec<u32> { cardinals(BASELAYER_APPID, None).into_iter().map(|n| n as u32).collect() }
/// The window pinned in front now, if any.
pub fn pinned() -> Option<u64> { cardinals(BASELAYER_WINDOW, None).first().copied().filter(|w| *w != 0) }
pub fn clear_pin() { remove_root(BASELAYER_WINDOW); }
/// Pins one window in front, whatever its number (a Steam question, an app picked in Quick Access, a sign-in page).
/// The numbers asked for stay: when that window closes, gamescope goes back to them.
pub fn focus_window(win: u64) {
    if win == 0 { return; }
    let _ = set_root(BASELAYER_WINDOW, win);
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
/// Hearth in front (its number; no window pinned).
pub fn clear_focus() { focus_ids(&[HEARTH_ID]); }

/// Which game to find a window for, and which windows are never it.
pub struct Want<'a> {
    /// The game's number (0: none, find it by its processes only).
    pub app_id: u32,
    /// Processes whose windows are never it: Hearth's own, the games put aside.
    pub exclude_pids: &'a [u64],
    /// Windows other games had.
    pub not_wins: &'a [u64],
    /// Numbers that are other games' (the games put aside) or Hearth's.
    pub other_ids: &'a [u32],
    /// The window this game had when it was last in front.
    pub known: u64,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Pick {
    /// A window carries the game's number: show the number.
    ById(u64),
    /// A window of the game's processes that doesn't carry its number yet: put it on, then show the number.
    ByProcess(u64),
    /// No window can be told to be the game's for sure: the window it had before, or the only one left. Pinned.
    Fallback(u64),
    Nothing,
}
fn real_pid(pid: u64) -> bool { pid > 0 && pid < u32::MAX as u64 }
/// The choice of what to show for a game, from gamescope's focusable windows (pure, for tests).
pub fn pick_window(wins: &[Win], want: &Want, pids: &[u32], steam_client: &dyn Fn(u64) -> bool) -> Pick {
    let app = want.app_id as u64;
    // Only the game's own windows carry its number (Xbox Cloud Gaming's is one of Hearth's windows, so this comes
    // before leaving out Hearth's processes).
    if want.app_id != 0 { if let Some(w) = wins.iter().find(|w| w.appid == app) { return Pick::ById(w.win); } }
    let ok: Vec<&Win> = wins.iter().filter(|w| {
        let other = w.appid != app && w.appid <= u32::MAX as u64 && want.other_ids.contains(&(w.appid as u32));
        !want.exclude_pids.contains(&w.pid) && !want.not_wins.contains(&w.win) && !other
    }).collect();
    if let Some(w) = ok.iter().find(|w| real_pid(w.pid) && pids.iter().any(|p| *p as u64 == w.pid)) { return Pick::ByProcess(w.win); }
    // The window it had before it was put aside (its processes weren't matched: a launcher's helper).
    if want.known != 0 { if let Some(w) = ok.iter().find(|w| w.win == want.known) { return Pick::Fallback(w.win); } }
    // Its processes not matched (some launchers hand the window to a helper): if only one window is left that isn't
    // Steam's, Hearth's or another game's, that's it. Not for Steam games (Steam's windows come and go).
    if !crate::appid::is_steam(want.app_id) {
        let rest: Vec<&&Win> = ok.iter().filter(|w| real_pid(w.pid) && !steam_client(w.pid)).collect();
        if rest.len() == 1 { return Pick::Fallback(rest[0].win); }
    }
    Pick::Nothing
}

/// A game's number in front: gamescope shows the best window carrying it (the game over its launcher or splash, as
/// on SteamOS). A game that minimized itself is asked to show again; that can map its window again, which without
/// Steam mode makes gamescope forget the number, so it goes back on.
pub fn front_by_id(app_id: u32, win: u64) {
    focus_ids(&focus_list(Some(app_id)));
    restore(win);
    if !steam_mode() { let _ = tag_window(win, app_id); }
}

/// Brings a game to the front once it has a window (games take a while to open one): by its number, putting it on
/// a window of its processes that doesn't carry it yet; only when no window can be told to be the game's, the window
/// it had before (`want.known`) or the only one left, pinned. Windows of games put aside are never picked. `still`
/// says whether this is still wanted. Its window when found.
pub fn focus_game(want: &Want, pids: &dyn Fn() -> Vec<u32>, still: &dyn Fn() -> bool) -> Option<u64> {
    if !available() { return None; }
    // Asked for right away (a window pinned now, like a Steam question, stays in front until it closes).
    if want.app_id != 0 { let _ = set_root_list(BASELAYER_APPID, &focus_list(Some(want.app_id))); }
    for _ in 0..20 {
        if !still() { return None; }
        let now = pids();
        let wins = focusable_windows();
        let pick = pick_window(&wins, want, &now, &|pid| crate::procs::is_steam_client(&proc_name(pid)));
        if pick != Pick::Nothing && !still() { return None; }
        match pick {
            Pick::ById(w) => {
                front_by_id(want.app_id, w);
                crate::bridge::log(&format!("gamescope: id {} in front (window {w:#x} carries it; {} processes)", want.app_id, now.len()));
                return Some(w);
            }
            Pick::ByProcess(w) => {
                if want.app_id != 0 && tag_window(w, want.app_id) {
                    front_by_id(want.app_id, w);
                    crate::bridge::log(&format!("gamescope: window {w:#x} (pid {}) gets id {}; id in front", wins.iter().find(|x| x.win == w).map(|x| x.pid).unwrap_or(0), want.app_id));
                } else {
                    focus_window(w);
                    crate::bridge::log(&format!("gamescope: window {w:#x} of the game's processes pinned in front (no id to put on it)"));
                }
                return Some(w);
            }
            Pick::Fallback(w) => {
                focus_window(w);
                crate::bridge::log(&format!("gamescope: no window carries id {} or belongs to its processes; window {w:#x} pinned in front", want.app_id));
                return Some(w);
            }
            Pick::Nothing => std::thread::sleep(Duration::from_millis(1500)),
        }
    }
    None
}

pub enum Ready { Window(u64), Dialog(u64, String) }

/// A Steam window that may need you while a game starts (a cloud-save conflict, an error, an EULA), not Steam's
/// own main, friends or "launching" windows, which Hearth keeps out of the way.
pub fn steam_question(title: &str) -> bool {
    let t = title.trim().to_lowercase();
    let steams_own = t.is_empty() || t == "steam" || t == "steamwebhelper" || t == "steam big picture mode" || t == "friends list"
        || t.starts_with("launching") || t.starts_with("preparing") || t.starts_with("updating") || t.starts_with("steam - updat");
    !steams_own
}

/// Waits until a game (or its launcher) has a real window up while Hearth shows its "starting" screen: a window
/// carrying the game's number, at least 640x360, seen twice in a row. A window of the game's processes that doesn't
/// carry it yet gets it here. A Steam window that stays up a few seconds while the game shows nothing may need you
/// (a cloud-save question): Dialog.
pub fn wait_for_window(app_id: u32, pids: &dyn Fn() -> Vec<u32>, exclude: &[u64], timeout: Duration, cancelled: &dyn Fn() -> bool) -> Option<Ready> {
    if !available() { return None; }
    let start = Instant::now();
    let mut last = 0;
    let mut steam_seen: std::collections::HashMap<u64, Instant> = Default::default();
    let app = app_id as u64;
    while start.elapsed() < timeout {
        if cancelled() { return None; }
        let want: Vec<u64> = pids().into_iter().map(u64::from).collect();
        // The number gamescope gives each window (from Steam's reaper, or STEAM_GAME), and its process as the X
        // server knows it (a sandboxed program's own _NET_WM_PID is a number inside its sandbox).
        let known: std::collections::HashMap<u64, (u64, u64)> = focusable_windows().into_iter().map(|w| (w.win, (w.appid, w.pid))).collect();
        let wins: Vec<Win> = mapped_windows().into_iter().map(|mut w| {
            if let Some((_, pid)) = known.get(&w.win).filter(|(_, p)| *p > 0 && *p < u32::MAX as u64) { w.pid = *pid; }
            w
        }).filter(|w| !exclude.contains(&w.pid)).collect();
        let mut game: Option<&Win> = None;
        for w in wins.iter().filter(|w| w.w >= 640 && w.h >= 360) {
            let carries = app_id != 0 && (known.get(&w.win).map(|k| k.0) == Some(app) || w.appid == app);
            if carries { game = Some(w); break; }
            if want.contains(&w.pid) {
                if app_id == 0 { game = Some(w); break; }
                if tag_window(w.win, app_id) {
                    crate::bridge::log(&format!("ids: window {:#x} \"{}\" (pid {}, {}) gets id {app_id}", w.win, w.title, w.pid, proc_name(w.pid)));
                    game = Some(w);
                    break;
                }
            }
        }
        match game { Some(g) if g.win == last => return Some(Ready::Window(g.win)), Some(g) => last = g.win, None => last = 0 }
        for w in &wins {
            let name = proc_name(w.pid);
            if (name == "steam" || name == "steamwebhelper") && w.w >= 480 && w.h >= 240 && !want.contains(&w.pid) && steam_question(&w.title) {
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

#[cfg(test)]
mod focus_tests {
    use super::*;
    use crate::appid::HEARTH_ID;
    fn w(win: u64, appid: u64, pid: u64) -> Win { Win { win, appid, pid, title: String::new(), w: 0, h: 0 } }
    const A: u32 = 0x8000_1000;   // put aside (frozen)
    const B: u32 = 0x8000_2000;   // the one coming to the front
    const ME: u64 = 50;           // Hearth

    #[test]
    fn asks_for_the_game_then_hearth() {
        assert_eq!(focus_list(Some(B)), vec![B, HEARTH_ID]);
        assert_eq!(focus_list(Some(620)), vec![620, HEARTH_ID]);
        assert_eq!(focus_list(None), vec![HEARTH_ID]);
        assert_eq!(focus_list(Some(0)), vec![HEARTH_ID]);
        assert_eq!(focus_list(Some(HEARTH_ID)), vec![HEARTH_ID]);
    }

    #[test]
    fn picks_the_games_own_window_never_a_frozen_one() {
        let no_steam = |_: u64| false;
        let frozen_pids = [ME, 300, 301];
        let others = [A, HEARTH_ID];
        let want = Want { app_id: B, exclude_pids: &frozen_pids, not_wins: &[], other_ids: &others, known: 0 };
        // Game A put aside (its windows carry A and its processes are frozen), Hearth's window, and B's launcher
        // and game window, both carrying B: B's number is shown (gamescope picks the best of B's windows).
        let wins = vec![w(0x100, A as u64, 300), w(0x101, A as u64, 301), w(0x200, HEARTH_ID as u64, ME), w(0x300, B as u64, 400), w(0x301, B as u64, 401)];
        assert_eq!(pick_window(&wins, &want, &[400, 401], &no_steam), Pick::ById(0x300));
        // B's window doesn't carry its number yet (gamescope without -e gives windows their own X id): found by B's
        // processes, then it gets the number.
        let wins = vec![w(0x100, A as u64, 300), w(0x200, HEARTH_ID as u64, ME), w(0x300, 0x300, 400)];
        assert_eq!(pick_window(&wins, &want, &[400], &no_steam), Pick::ByProcess(0x300));
        // A frozen window is never picked, even when it's the only other window, or B's processes list it by mistake.
        let wins = vec![w(0x100, A as u64, 300), w(0x200, HEARTH_ID as u64, ME)];
        assert_eq!(pick_window(&wins, &want, &[300], &no_steam), Pick::Nothing);
        // A frozen game's window carrying its number is skipped even with a process that isn't listed as frozen
        // (a Wine helper the guesses missed).
        let wins = vec![w(0x100, A as u64, 999), w(0x200, HEARTH_ID as u64, ME)];
        assert_eq!(pick_window(&wins, &want, &[], &no_steam), Pick::Nothing);
        // Nothing carries B and its processes aren't matched: the window it had before, else the only one left.
        let want_known = Want { known: 0x500, ..want };
        let wins = vec![w(0x100, A as u64, 300), w(0x500, 0x500, 700), w(0x600, 0x600, 701)];
        assert_eq!(pick_window(&wins, &want_known, &[], &no_steam), Pick::Fallback(0x500));
        let want = Want { app_id: B, exclude_pids: &frozen_pids, not_wins: &[], other_ids: &others, known: 0 };
        let wins = vec![w(0x100, A as u64, 300), w(0x600, 0x600, 701)];
        assert_eq!(pick_window(&wins, &want, &[], &no_steam), Pick::Fallback(0x600));
        // ...but not Steam's own window, and never for a Steam game.
        assert_eq!(pick_window(&wins, &want, &[], &|pid| pid == 701), Pick::Nothing);
        let steam_game = Want { app_id: 620, exclude_pids: &frozen_pids, not_wins: &[], other_ids: &others, known: 0 };
        assert_eq!(pick_window(&wins, &steam_game, &[], &no_steam), Pick::Nothing);
        // A Steam game: its windows carry its Steam app id (gamescope reads it from Steam's reaper).
        let wins = vec![w(0x100, A as u64, 300), w(0x700, 620, 800)];
        assert_eq!(pick_window(&wins, &steam_game, &[], &no_steam), Pick::ById(0x700));
        // Xbox Cloud Gaming: its window is one of Hearth's (Hearth's process is left out) but carries the game's number.
        let cloud = Want { app_id: B, exclude_pids: &frozen_pids, not_wins: &[], other_ids: &others, known: 0 };
        assert_eq!(pick_window(&[w(0x200, HEARTH_ID as u64, ME), w(0x210, B as u64, ME)], &cloud, &[], &no_steam), Pick::ById(0x210));
        // Windows other games had are skipped too.
        let want_not = Want { app_id: B, exclude_pids: &frozen_pids, not_wins: &[0x600], other_ids: &others, known: 0 };
        assert_eq!(pick_window(&[w(0x600, 0x600, 701)], &want_not, &[701], &no_steam), Pick::Nothing);
    }
}

#[cfg(test)]
mod steam_window_tests {
    use super::steam_question;
    #[test]
    fn only_steam_questions_come_in_front() {
        for t in ["Steam", "Friends List", "Launching...", "Preparing to launch Portal 2...", "Steam - Updating", "Steam Big Picture Mode", ""] { assert!(!steam_question(t), "{t}"); }
        for t in ["Steam Cloud Sync Conflict", "Sign in to Steam", "Steam - Error", "Install - Portal 2"] { assert!(steam_question(t), "{t}"); }
    }
}
