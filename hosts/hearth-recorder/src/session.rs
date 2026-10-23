//! A recording, from start to saved file: `hearth-recorder --session <request>` runs this in a process of its
//! own (started detached by lib.rs's start), so it goes on when the window closes and the shortcuts can stop it.
//! It works out the plan (plan.rs), starts its programs, watches them and control.rs's requests, then stops them
//! properly (SIGINT: the file is finished, not cut off), saves, and says so with a notification.

use crate::control::{self, Last, State};
use crate::plan::{self, Kind, Paths, Plan, Proc, Req};
use crate::probe::Env;
use hearth_media::{dirs, notify, pulse, tools};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const APP: &str = "Recorder";
pub const ICON: &str = "hearth-recorder";

static STOP: AtomicBool = AtomicBool::new(false);
static SAVE: AtomicBool = AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn on_stop(_: libc::c_int) { STOP.store(true, Ordering::SeqCst); }
#[cfg(unix)]
extern "C" fn on_save(_: libc::c_int) { SAVE.store(true, Ordering::SeqCst); }

fn handle_signals() {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGINT, on_stop as libc::sighandler_t);
        libc::signal(libc::SIGTERM, on_stop as libc::sighandler_t);
        libc::signal(libc::SIGUSR1, on_save as libc::sighandler_t);
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
    }
}

/// ~/Videos/Recordings
pub fn recordings_dir() -> PathBuf { dirs::videos().join("Recordings") }
/// ~/Music/Recordings (the sound recorder's)
pub fn sounds_dir() -> PathBuf { dirs::music().join("Recordings") }

fn what(k: Kind) -> &'static str { match k { Kind::Record => "Recording", Kind::Replay => "Instant replay", Kind::Sound => "Sound recording" } }

pub fn duration_text(ms: u64) -> String {
    let s = ms / 1000;
    if s < 60 { format!("{s} s") } else if s < 3600 { format!("{} min {} s", s / 60, s % 60) } else { format!("{} h {} min", s / 3600, (s / 60) % 60) }
}

/// Runs a whole recording. The exit code is 0 when it ended well.
pub fn run(req: Req) -> i32 {
    handle_signals();
    let me = std::process::id();
    if let Some(s) = control::current() {
        if s.pid != me { fail(&req, "Something is already being recorded"); return 1; }
    }
    control::clear_requests();
    let mut st = State { kind: req.kind.name().into(), status: "starting".into(), pid: me, started: dirs::now_ms(),
                         area: req.area.is_some(), replay_secs: req.replay_secs, ..Default::default() };
    control::write(&st);
    let r = record(&req, &mut st);
    control::clear();
    match r {
        Ok(Some(f)) => {
            let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            control::set_last(&Last { kind: req.kind.name().into(), ok: true, file: f.to_string_lossy().into(), error: String::new(), time: dirs::now_ms() });
            notify::saved(APP, ICON, &format!("{} saved", what(req.kind)), &format!("{name} · {}", duration_text(dirs::now_ms().saturating_sub(st.started))), &f);
            0
        }
        Ok(None) => {
            control::set_last(&Last { kind: req.kind.name().into(), ok: true, time: dirs::now_ms(), ..Default::default() });
            if req.announce { notify::brief(APP, ICON, "Instant replay is off", "Super+Alt+G turns it on again"); }
            0
        }
        Err(e) => { fail(&req, &e); 1 }
    }
}

fn fail(req: &Req, e: &str) {
    control::set_last(&Last { kind: req.kind.name().into(), ok: false, file: String::new(), error: e.into(), time: dirs::now_ms() });
    notify::info(APP, ICON, &format!("{} stopped", what(req.kind)), e);
}

fn spawn(p: &Proc, log: &std::fs::File, piped: bool) -> Result<Child, String> {
    let err = log.try_clone().map(Stdio::from).unwrap_or_else(|_| Stdio::null());
    let out = if piped { Stdio::piped() } else { log.try_clone().map(Stdio::from).unwrap_or_else(|_| Stdio::null()) };
    Command::new(&p.prog).args(&p.args).stdin(Stdio::null()).stdout(out).stderr(err).spawn()
        .map_err(|e| format!("Couldn't start {}: {e}", p.prog))
}

/// The sound recorder's level meter: the loudest sample of each tenth of a second.
fn level_reader(mut out: std::process::ChildStdout, lv: Arc<AtomicU32>) {
    std::thread::spawn(move || {
        let mut buf = vec![0u8; 1600];
        while out.read_exact(&mut buf).is_ok() {
            let peak = buf.chunks_exact(2).map(|c| (i16::from_le_bytes([c[0], c[1]]) as i32).abs()).max().unwrap_or(0);
            lv.store((peak as f32 / 32768.0).to_bits(), Ordering::SeqCst);
        }
    });
}

fn log_tail() -> String {
    let t = tools::tail(&control::log_file(), 6);
    let t: Vec<&str> = t.lines().filter(|l| !l.starts_with("$ ")).collect();
    t.join(" · ").chars().take(600).collect()
}

fn record(req: &Req, st: &mut State) -> Result<Option<PathBuf>, String> {
    let env = Env::detect(false);
    let backend = plan::backend(&env, req)?;
    let sound = req.kind == Kind::Sound;
    let (want_sys, want_mic) = if sound { (false, true) } else { (req.system, req.mic) };
    // gpu-screen-recorder takes its sound itself; the others get one PulseAudio source (a mix when it's both).
    let (audio, _mix) = if backend == "gsr" { (None, None) } else {
        match pulse::source_for(want_sys, want_mic) {
            Ok(x) => x,
            Err(e) if sound => return Err(e),
            Err(_) => pulse::source_for(want_sys, false).unwrap_or((None, None)),
        }
    };
    st.audio = match (want_sys, want_mic && (backend == "gsr" || audio.is_some())) {
        (true, true) => "What's playing and the microphone", (true, false) => "What's playing", (false, true) => "Microphone", _ => "No sound",
    }.into();
    if !sound && backend != "gsr" && audio.is_none() { st.audio = "No sound".into(); }

    let rec_dir = recordings_dir();
    let file = match req.kind {
        Kind::Sound => dirs::unique(&sounds_dir(), "Sound recording", plan::sound_ext(&env, req, backend)),
        Kind::Record => dirs::unique(&rec_dir, if req.area.is_some() { "Area recording" } else { "Recording" }, "mp4"),
        Kind::Replay => rec_dir.join("replay.mp4"),
    };
    let _ = std::fs::create_dir_all(&rec_dir);
    let work = dirs::cache("hearth-recorder").join("replay");
    let _ = std::fs::remove_dir_all(&work);
    if req.kind == Kind::Replay { std::fs::create_dir_all(work.join("pieces")).map_err(|e| format!("Couldn't make {}: {e}", work.display()))?; }
    let pl = plan::plan(&env, req, audio.as_deref(), &Paths { file: file.clone(), work: work.clone(), replay_dir: rec_dir.clone() })?;
    st.backend = pl.backend.clone();
    st.encoder = pl.encoder.clone();
    if let Some(f) = &pl.file { st.file = f.to_string_lossy().into(); }
    if let Some(f) = &pl.fifo { tools::mkfifo(f)?; }

    let mut log = std::fs::File::create(control::log_file()).map_err(|e| format!("Couldn't write the log: {e}"))?;
    for p in &pl.procs { let _ = writeln!(log, "$ {}", p.line()); }

    // The readers first (they wait at the pipe), then the capture.
    let n = pl.procs.len();
    let mut kids: Vec<Option<Child>> = (0..n).map(|_| None).collect();
    let level = Arc::new(AtomicU32::new(0));
    let mut started_err: Option<String> = None;
    for i in (0..n).rev() {
        match spawn(&pl.procs[i], &log, i == 0 && pl.level) {
            Ok(mut c) => {
                if i == 0 && pl.level { if let Some(o) = c.stdout.take() { level_reader(o, level.clone()); } }
                kids[i] = Some(c);
            }
            Err(e) => { started_err = Some(e); break; }
        }
    }
    let mut kids: Vec<Child> = kids.into_iter().flatten().collect();
    if started_err.is_none() && kids.len() != n { started_err = Some("Couldn't start the recording".into()); }

    // It has a moment to fail (no screen, the encoder refused...).
    let mut err = started_err;
    if err.is_none() {
        let until = Instant::now() + Duration::from_millis(1500);
        while Instant::now() < until && err.is_none() {
            if kids.iter_mut().any(|k| matches!(k.try_wait(), Ok(Some(_)))) { err = Some(format!("It stopped straight away: {}", log_tail())); }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    if err.is_none() {
        st.status = "recording".into();
        st.started = dirs::now_ms();
        control::write(st);
        if req.announce {
            match req.kind {
                Kind::Record => notify::brief(APP, ICON, "Recording", "Super+Alt+R stops it"),
                Kind::Replay => notify::brief(APP, ICON, "Instant replay is on", &format!("Super+Alt+G saves the last {}", duration_text(req.replay_secs as u64 * 1000))),
                Kind::Sound => {}
            }
        }
        let mut last_write = Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(100));
            if STOP.load(Ordering::SeqCst) || control::take("stop") { break; }
            if SAVE.swap(false, Ordering::SeqCst) | control::take("save") && req.kind == Kind::Replay {
                st.status = "saving".into();
                control::write(st);
                match save_replay(&pl, &kids, req.replay_secs, &rec_dir) {
                    Ok(f) => {
                        st.saves += 1;
                        st.last_save = f.to_string_lossy().into();
                        let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        notify::saved(APP, ICON, "Replay saved", &format!("The last {}: {name}", duration_text(req.replay_secs as u64 * 1000)), &f);
                    }
                    Err(e) => {
                        st.saves += 1;
                        st.last_save = String::new();
                        notify::info(APP, ICON, "Couldn't save the replay", &e);
                    }
                }
                st.status = "recording".into();
                control::write(st);
            }
            if let Some(k) = kids.iter_mut().find(|k| matches!(k.try_wait(), Ok(Some(_)))) {
                let _ = k;
                err = Some(format!("The recording stopped by itself: {}", log_tail()));
                break;
            }
            if pl.level && last_write.elapsed() > Duration::from_millis(150) {
                st.level = f32::from_bits(level.load(Ordering::SeqCst));
                control::write(st);
                last_write = Instant::now();
            }
        }
    }

    // Stop: the capture first (it finishes its file), then the readers (they end with the pipe).
    st.status = "stopping".into();
    control::write(st);
    if let Some(first) = kids.first_mut() { tools::stop_child(first, Duration::from_secs(20)); }
    for k in kids.iter_mut().skip(1) {
        if !tools::wait_child(k, Duration::from_secs(8)) { tools::stop_child(k, Duration::from_secs(5)); }
    }
    let _ = std::fs::remove_dir_all(&work);

    match &pl.file {
        Some(f) => {
            let size = std::fs::metadata(f).map(|m| m.len()).unwrap_or(0);
            if size > 0 { Ok(Some(f.clone())) }
            else {
                let _ = std::fs::remove_file(f);
                Err(err.unwrap_or_else(|| format!("Nothing was recorded. {}", log_tail())))
            }
        }
        None => match err { Some(e) => Err(e), None => Ok(None) },
    }
}

fn media_files(dir: &Path, ext: &str) -> Vec<(u64, PathBuf)> {
    std::fs::read_dir(dir).into_iter().flatten().flatten().filter_map(|e| {
        let p = e.path();
        if p.extension().map(|x| x.to_string_lossy().to_ascii_lowercase()) != Some(ext.to_string()) { return None; }
        let m = e.metadata().ok()?;
        if m.len() == 0 { return None; }
        let t = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis() as u64;
        Some((t, p))
    }).collect()
}

/// Instant replay → a file: gpu-screen-recorder saves its own (SIGUSR1); otherwise the newest pieces are joined.
fn save_replay(pl: &Plan, kids: &[Child], secs: u32, rec_dir: &Path) -> Result<PathBuf, String> {
    if let Some(dir) = &pl.gsr_dir {
        let before: Vec<PathBuf> = media_files(dir, "mp4").into_iter().map(|(_, p)| p).collect();
        let pid = kids.first().map(|k| k.id()).unwrap_or(0);
        if !tools::signal(pid, tools::SIGUSR1) { return Err("gpu-screen-recorder isn't running".into()); }
        let until = Instant::now() + Duration::from_secs(25);
        let mut seen: Option<(PathBuf, u64)> = None;
        let found = loop {
            if Instant::now() > until { return Err("gpu-screen-recorder didn't save the replay".into()); }
            std::thread::sleep(Duration::from_millis(300));
            let mut new: Vec<(u64, PathBuf)> = media_files(dir, "mp4").into_iter().filter(|(_, p)| !before.contains(p)).collect();
            new.sort();
            let Some((_, f)) = new.pop() else { continue };
            let len = std::fs::metadata(&f).map(|m| m.len()).unwrap_or(0);
            if let Some((p, l)) = &seen { if *p == f && *l == len && len > 0 { break f; } }
            seen = Some((f, len));
        };
        let dest = dirs::unique(rec_dir, "Replay", "mp4");
        return Ok(if std::fs::rename(&found, &dest).is_ok() { dest } else { found });
    }
    let segdir = pl.segments.as_ref().ok_or("Instant replay has nothing to save")?;
    let pieces = plan::pieces_for(media_files(segdir, "ts"), secs);
    if pieces.is_empty() { return Err("Instant replay hasn't recorded anything yet. Try again in a few seconds.".into()); }
    let list = segdir.with_file_name("list.txt");
    std::fs::write(&list, plan::concat_list(&pieces)).map_err(|e| e.to_string())?;
    let dest = dirs::unique(rec_dir, "Replay", "mp4");
    let (l, d) = (list.to_string_lossy().to_string(), dest.to_string_lossy().to_string());
    let o = tools::run("ffmpeg", &["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-f", "concat", "-safe", "0", "-i", &l,
                                   "-map", "0", "-c", "copy", "-movflags", "+faststart", &d], Duration::from_secs(90))?;
    let size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    if o.ok() && size > 0 { Ok(dest) } else {
        let _ = std::fs::remove_file(&dest);
        Err(format!("Couldn't join the replay: {}", o.stderr.lines().last().unwrap_or("")))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn durations() {
        assert_eq!(super::duration_text(4_200), "4 s");
        assert_eq!(super::duration_text(30_000), "30 s");
        assert_eq!(super::duration_text(300_000), "5 min 0 s");
        assert_eq!(super::duration_text(3_725_000), "1 h 2 min");
    }
}
