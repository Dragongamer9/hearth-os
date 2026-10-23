//! Hearth Recorder: Hearth OS's screen recorder and sound recorder.
//!  - Record the whole screen or an area (Desktop Mode), or the game (Game Mode), with what's playing and/or the
//!    microphone; on the graphics card (NVENC on Nvidia, VA-API on AMD and Intel) or the processor.
//!  - Instant replay: keeps the last 30 s / 1 min / 5 min, saved when you ask (Super+Alt+G, Quick Access).
//!  - Sound recorder: the microphone to Opus or MP3.
//! Recordings go to ~/Videos/Recordings (sound: ~/Music/Recordings) with a notification (Open, Show in folder).
//! The window (main.rs), the shortcuts (`hearth-recorder --toggle`...) and Hearth's Quick Access all go through
//! the functions here; the recording itself runs in a process of its own (session.rs).

pub mod control;
pub mod plan;
pub mod probe;
pub mod session;
pub mod settings;

use control::State;
use plan::{Kind, Rect, Req};
use serde_json::{json, Value};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The recording that's going (null when none), how the last one ended, and the settings.
pub fn status() -> Value {
    json!({"session": control::current(), "last": control::last(), "settings": settings::load()})
}

/// Starts a recording ("record", "replay" or "sound"), optionally of an area; returns once it's really
/// recording (or with why it couldn't).
pub fn start(kind: Kind, area: Option<Rect>, announce: bool) -> Result<State, String> {
    if let Some(s) = control::current() {
        return Err(match s.kind.as_str() { "replay" => "Instant replay is on. Turn it off first.", "sound" => "The sound recorder is recording.", _ => "Already recording." }.into());
    }
    let mut req = Req::from_settings(kind, &settings::load());
    req.area = area;
    req.announce = announce;
    control::clear_last();
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut c = Command::new(exe);
    c.arg("--session").arg(serde_json::to_string(&req).map_err(|e| e.to_string())?)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = hearth_media::tools::spawn_detached(&mut c).map_err(|e| format!("Couldn't start recording: {e}"))?;
    let pid = child.id();
    // Collected when it ends, so a window that started it doesn't keep a finished process around.
    std::thread::spawn(move || { let _ = child.wait(); });
    // The first time it tries the encoders, which takes a few seconds.
    let until = Instant::now() + Duration::from_secs(40);
    while Instant::now() < until {
        if let Some(s) = control::current() { if s.pid == pid && s.status == "recording" { return Ok(s); } }
        if !hearth_media::tools::alive(pid) {
            return Err(control::last().filter(|l| !l.ok).map(|l| l.error).unwrap_or_else(|| "The recording stopped straight away".into()));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    control::current().ok_or_else(|| "The recording didn't start".into())
}

/// Stops what's recording; returns how it ended ({ok, file, error}).
pub fn stop() -> Result<Value, String> {
    let s = control::current().ok_or("Nothing is being recorded")?;
    control::request("stop");
    let until = Instant::now() + Duration::from_secs(45);
    while hearth_media::tools::alive(s.pid) && Instant::now() < until { std::thread::sleep(Duration::from_millis(100)); }
    if hearth_media::tools::alive(s.pid) { return Err("The recording is taking a long time to stop".into()); }
    let l = control::last().unwrap_or_default();
    if l.ok { Ok(json!({"ok": true, "kind": s.kind, "file": l.file})) } else { Err(if l.error.is_empty() { "The recording ended without saving".into() } else { l.error }) }
}

/// Instant replay: saves the last stretch now; returns the file.
pub fn save_replay() -> Result<String, String> {
    let s = control::current().filter(|s| s.kind == "replay").ok_or("Instant replay isn't on")?;
    let before = s.saves;
    control::request("save");
    let until = Instant::now() + Duration::from_secs(60);
    while Instant::now() < until {
        std::thread::sleep(Duration::from_millis(150));
        match control::current() {
            Some(n) if n.saves > before => return if n.last_save.is_empty() {
                Err(if n.last_error.is_empty() { "Couldn't save the replay".into() } else { format!("Couldn't save the replay: {}", n.last_error) })
            } else { Ok(n.last_save) },
            Some(_) => {}
            None => return Err("Instant replay stopped".into()),
        }
    }
    Err("Saving the replay is taking a long time".into())
}

/// The shortcut for recording (Super+Alt+R, Quick Access): starts the whole screen, or stops and saves.
/// Instant replay is turned off first if it's on (one capture at a time).
pub fn toggle(area: Option<Rect>, announce: bool) -> Result<Value, String> {
    if let Some(s) = control::current() {
        if s.kind == "record" { return stop(); }
        if s.kind == "replay" { let _ = stop(); }
        if s.kind == "sound" { return Err("The sound recorder is recording. Stop it first.".into()); }
    }
    start(Kind::Record, area, announce).map(|s| json!({"ok": true, "started": s}))
}

/// The instant replay shortcut (Super+Alt+G, Quick Access): saves when it's on, turns it on when it isn't.
pub fn replay_save_or_start(announce: bool) -> Result<Value, String> {
    match control::current() {
        Some(s) if s.kind == "replay" => save_replay().map(|f| json!({"ok": true, "saved": f})),
        Some(_) => Err("Recorder is already recording. Stop it first.".into()),
        None => start(Kind::Replay, None, announce).map(|s| json!({"ok": true, "started": s})),
    }
}

/// Lets you drag a box around what to record (slurp, Desktop Mode). None when you pressed Esc.
pub fn choose_area() -> Result<Option<Rect>, String> {
    if !hearth_media::tools::have("slurp") { return Err("Choosing an area needs slurp, which isn't installed".into()); }
    let o = hearth_media::tools::run("slurp", &["-d", "-b", "#00000066", "-c", "#ff8a3d", "-s", "#ff8a3d22", "-w", "2", "-f", "%x,%y %wx%h"], Duration::from_secs(300))?;
    if !o.ok() { return Ok(None); }
    Ok(Rect::parse(o.stdout.trim()))
}

/// What the window lists: recordings and sound recordings, newest first.
pub fn files() -> Value {
    json!({"videos": hearth_media::files::list(&session::recordings_dir(), hearth_media::files::VIDEO, 60),
           "sounds": hearth_media::files::list(&session::sounds_dir(), hearth_media::files::AUDIO, 60),
           "videoDir": session::recordings_dir().to_string_lossy(), "soundDir": session::sounds_dir().to_string_lossy()})
}

pub fn dirs() -> Vec<std::path::PathBuf> { vec![session::recordings_dir(), session::sounds_dir()] }
