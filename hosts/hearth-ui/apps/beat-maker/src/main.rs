//! Hearth Beat Maker, in pure Rust (hearth-ui, iced): the approved board docs/mockups/open-source-apps/18-beat-maker.png
//! (checklist docs/mockups/checklists/open-source-18-beat-maker.md). Built on LMMS (GPL-2): a song from patterns
//! (LMMS's Song Editor and Beat+Bassline), the piano roll, instruments and their knobs, saved as an LMMS project
//! (.mmp) beside Hearth's own file, opened in LMMS and rendered by LMMS's command line (`lmms render`) when you ask.
//! Playing as you go is Hearth's own synth (synth.rs) through PipeWire (pw-play; paplay or aplay where it isn't),
//! so it plays the moment you press ▶. ● records your mic (pw-record) onto the Voice-over track.
//!
//! The song is saved as you change it (~/Music/Hearth Beat Maker/<name>.hbm.json, settings_file's safe write with a
//! .bak). Export for Hearth's videos: .wav, .mp3 (ffmpeg), stems, the beats for video cuts, the song's source into
//! the hearth-workspace repository and the hearthos.fun music list (your hearth-os folder), never committed for you.
//! Nothing at rest: timers only while playing or recording.

mod song;
mod synth;
mod view;

use hearth_ui::app::{self, win};
use hearth_ui::{run, settings_file, Pal};
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::{Element, Subscription, Task};
use serde_json::{json, Map, Value};
use song::{Body, Clip, Note, Pattern, Row, Song, Track};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

pub fn music_dir() -> PathBuf {
    hearth_ui::paths::home().join("Music/Hearth Beat Maker")
}
pub fn prefs_path() -> PathBuf {
    hearth_ui::paths::home().join(".config/hearth/beat-maker.json")
}
pub fn slug(s: &str) -> String {
    let v: String = s.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect();
    v.split('-').filter(|x| !x.is_empty()).collect::<Vec<_>>().join("-")
}
/// LMMS's data (its presets and samples): the system's, or what HEARTH_LMMS_DATA says (CI).
pub fn lmms_data() -> PathBuf {
    std::env::var_os("HEARTH_LMMS_DATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/usr/share/lmms"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Draw,
    Pick,
    Cut,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Export {
    pub name: String,
    pub wav: bool,
    pub mp3: bool,
    pub stems: bool,
    pub site: bool,
    pub beats: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Dlg {
    Library(String),
    Key,
    Chords,
    Said(String, String),
    NoLmms,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Play,
    Stop,
    ToStart,
    Record,
    Rendered(u64, Result<PathBuf, String>),
    Frame,
    Seek(f32),
    Bpm(i32),
    Sig,
    Key(u8, bool),
    Mode(Mode),
    PickTrack(usize),
    Mute(usize),
    Solo(usize),
    /// a press on a track's lane at this bar (Draw adds, Pick picks, Cut cuts)
    Lane(usize, f32),
    MoveClip(usize, usize, f32),
    Step(usize, usize),
    StepVel(usize, usize, u8),
    Swing,
    AddRow,
    Grid(f32),
    Fit,
    AddNote(i32, f32),
    PickNote(usize, bool),
    MoveNote(usize, i32, f32),
    SizeNote(usize, f32),
    DelNote(usize),
    NoteVel(usize, u8),
    RollScroll(i32),
    MakeChords(&'static str),
    Knob(usize, f32),
    Find(String),
    Open(Dlg),
    Close,
    AddInst(usize),
    AddSample(String),
    Effect(&'static str),
    ExportOpen,
    ExportName(String),
    ExportToggle(&'static str),
    ExportGo,
    Exported(Result<String, String>),
    OpenLmms,
    InstallLmms,
    Installed(run::Output),
    LmmsRendered(run::Output),
    Cpu,
    KeyEv(KEvent),
    Drag,
    Max,
    Min,
    Quit,
    Resize(iced::window::Direction),
}

pub struct App {
    pub describe: hearth_ui::describe::Describe,
    pub p: Pal,
    pub song: Song,
    pub file: PathBuf,
    pub sel_track: usize,
    pub sel_clip: Option<(usize, usize)>,
    pub beat_pat: Option<usize>,
    pub notes_pat: Option<usize>,
    pub notes_track: usize,
    pub sel_notes: Vec<usize>,
    pub mode: Mode,
    pub grid: f32,
    pub fit: bool,
    pub roll_top: i32,
    pub pos: f32,
    pub playing: Option<(Instant, f32)>,
    pub rendering: u64,
    pub recording: Option<(Instant, f32, PathBuf)>,
    pub cpu: f32,
    pub cpu_last: Option<(Instant, u64)>,
    pub export: Option<Export>,
    pub dlg: Option<Dlg>,
    pub find: String,
    pub said: String,
    pub busy: String,
}

static PLAYER: OnceLock<Mutex<Option<std::process::Child>>> = OnceLock::new();
static RECORDER: OnceLock<Mutex<Option<std::process::Child>>> = OnceLock::new();

fn slot(s: &'static OnceLock<Mutex<Option<std::process::Child>>>) -> &'static Mutex<Option<std::process::Child>> {
    s.get_or_init(|| Mutex::new(None))
}

fn stop_child(s: &'static OnceLock<Mutex<Option<std::process::Child>>>, gentle: bool) {
    if let Ok(mut g) = slot(s).lock() {
        if let Some(mut c) = g.take() {
            if gentle {
                // pw-record and arecord finish their file on an interrupt
                unsafe {
                    libc::kill(c.id() as i32, libc::SIGINT);
                }
                for _ in 0..30 {
                    if matches!(c.try_wait(), Ok(Some(_))) {
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

fn on_path(c: &str) -> bool {
    std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(c).is_file())).unwrap_or(false)
}

pub fn lmms_cmd() -> Option<Vec<String>> {
    if on_path("lmms") {
        Some(vec!["lmms".into()])
    } else if hearth_ui::paths::home().join(".local/share/flatpak/app/io.lmms.LMMS").is_dir() || Path::new("/var/lib/flatpak/app/io.lmms.LMMS").is_dir() {
        Some(vec!["flatpak".into(), "run".into(), "io.lmms.LMMS".into()])
    } else {
        None
    }
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let prefs = settings_file::read(&prefs_path()).map().unwrap_or_default();
        let file = app::arg("--open").map(PathBuf::from).or_else(|| prefs.get("last").and_then(Value::as_str).map(PathBuf::from)).unwrap_or_else(|| music_dir().join("my-first-song.hbm.json"));
        let song = settings_file::read(&file).map().and_then(|m| song::from_json(&Value::Object(m))).unwrap_or_else(|| song::blank("My first song"));
        let mut a = App {
            describe: hearth_ui::describe::Describe::new("beat-maker", "Hearth Beat Maker").with_pages(view::BLOCKS).with_kit(&view::KIT),
            p: hearth_ui::theme::load(),
            song,
            file,
            sel_track: 0,
            sel_clip: None,
            beat_pat: None,
            notes_pat: None,
            notes_track: 0,
            sel_notes: Vec::new(),
            mode: Mode::Draw,
            grid: 0.5,
            fit: true,
            roll_top: 72,
            pos: app::arg("--at").and_then(|s| s.parse().ok()).unwrap_or(0.0),
            playing: None,
            rendering: 0,
            recording: None,
            cpu: 0.0,
            cpu_last: None,
            export: None,
            dlg: None,
            find: String::new(),
            said: String::new(),
            busy: String::new(),
        };
        a.default_editors();
        if let Some(t) = app::arg("--track").and_then(|n| a.song.tracks.iter().position(|t| t.name == n)) {
            a.sel_track = t;
        }
        if let Some(n) = app::arg("--notes").and_then(|n| a.song.patterns.iter().position(|p| p.name == n)) {
            a.notes_pat = Some(n);
            if let Some(t) = a.song.tracks.iter().position(|t| t.clips.iter().any(|c| c.pat == n)) {
                a.notes_track = t;
            }
        }
        if let Some(n) = app::arg("--beat").and_then(|n| a.song.patterns.iter().position(|p| p.name == n)) {
            a.beat_pat = Some(n);
        }
        if let Some(sel) = app::arg("--select-notes") {
            a.sel_notes = sel.split(',').filter_map(|x| x.parse().ok()).collect();
        }
        match app::arg("--show").as_deref() {
            Some("export") => a.export = Some(a.new_export()),
            Some("library") => a.dlg = Some(Dlg::Library("Synths".into())),
            _ => {}
        }
        a.cpu_tick();
        (a, Task::none())
    }

    fn new_export(&self) -> Export {
        Export { name: self.song.name.clone(), wav: true, mp3: true, stems: false, site: true, beats: true }
    }

    /// The Beat and Notes panes show the first beat and the first notes the song has, until you pick a clip.
    fn default_editors(&mut self) {
        if self.beat_pat.is_none() {
            self.beat_pat = self.song.patterns.iter().position(|p| matches!(p.body, Body::Beat { .. }));
        }
        if self.notes_pat.is_none() {
            for (ti, t) in self.song.tracks.iter().enumerate() {
                if let Some(c) = t.clips.iter().find(|c| matches!(self.song.patterns.get(c.pat).map(|p| &p.body), Some(Body::Notes(_)))) {
                    self.notes_pat = Some(c.pat);
                    self.notes_track = ti;
                    break;
                }
            }
        }
    }

    /// Saved as it changes (the song's own file, the safe way), and which song was last open.
    fn save(&mut self) {
        if let Some(d) = self.file.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let text = serde_json::to_string_pretty(&song::to_json(&self.song)).unwrap_or_default();
        if let Err(e) = settings_file::write_bytes(&self.file, text.as_bytes(), song::valid) {
            self.said = format!("The song couldn't be saved: {e}");
        }
        let f = self.file.display().to_string();
        if let Some(d) = prefs_path().parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = settings_file::update_json(&prefs_path(), move |m: &mut Map<String, Value>| {
            if m.get("last").and_then(Value::as_str) != Some(f.as_str()) {
                m.insert("last".into(), Value::String(f));
            }
        });
    }

    pub fn bar_at(&self, secs: f32) -> f32 {
        secs / self.song.sec_per_beat() / self.song.per_bar()
    }

    fn cpu_tick(&mut self) {
        let Ok(st) = std::fs::read_to_string("/proc/self/stat") else { return };
        let f: Vec<&str> = st.rsplit(')').next().unwrap_or("").split_whitespace().collect();
        let ticks = f.get(11).and_then(|x| x.parse::<u64>().ok()).unwrap_or(0) + f.get(12).and_then(|x| x.parse::<u64>().ok()).unwrap_or(0);
        let now = Instant::now();
        if let Some((at, was)) = self.cpu_last {
            let secs = now.duration_since(at).as_secs_f32();
            if secs > 0.5 {
                let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as f32;
                self.cpu = (ticks.saturating_sub(was)) as f32 / hz / secs * 100.0;
                self.cpu_last = Some((now, ticks));
            }
        } else {
            self.cpu_last = Some((now, ticks));
        }
    }

    pub fn notes(&self) -> Option<&Vec<Note>> {
        match self.song.patterns.get(self.notes_pat?).map(|p| &p.body) {
            Some(Body::Notes(n)) => Some(n),
            _ => None,
        }
    }
    fn notes_mut(&mut self) -> Option<&mut Vec<Note>> {
        let i = self.notes_pat?;
        match self.song.patterns.get_mut(i).map(|p| &mut p.body) {
            Some(Body::Notes(n)) => Some(n),
            _ => None,
        }
    }
    fn beat_rows_mut(&mut self) -> Option<(&mut Vec<Row>, &mut usize, &mut f32)> {
        let i = self.beat_pat?;
        match self.song.patterns.get_mut(i).map(|p| &mut p.body) {
            Some(Body::Beat { rows, steps, swing }) => Some((rows, steps, swing)),
            _ => None,
        }
    }

    /// The song's sounds counted for the Library: (name, how many).
    pub fn library(&self) -> Vec<(String, String)> {
        let d = lmms_data();
        let count = |dir: PathBuf, exts: &[&str]| -> usize {
            fn walk(p: &Path, exts: &[&str], n: &mut usize, depth: u32) {
                if depth > 6 {
                    return;
                }
                if let Ok(rd) = std::fs::read_dir(p) {
                    for e in rd.flatten() {
                        let q = e.path();
                        if q.is_dir() {
                            walk(&q, exts, n, depth + 1);
                        } else if q.extension().map(|x| exts.contains(&x.to_string_lossy().to_lowercase().as_str())).unwrap_or(false) {
                            *n += 1;
                        }
                    }
                }
            }
            let mut n = 0;
            walk(&dir, exts, &mut n, 0);
            n
        };
        let presets = song::presets();
        let synths = presets.iter().filter(|p| !matches!(p.kind, song::Kind::Drums)).count() + count(d.join("presets"), &["xpf"]);
        let kits = presets.iter().filter(|p| matches!(p.kind, song::Kind::Drums)).count() + std::fs::read_dir(d.join("samples/drums")).map(|r| r.flatten().filter(|e| e.path().is_dir()).count()).unwrap_or(0);
        let samples = count(d.join("samples"), &["ogg", "wav", "flac"]);
        let recs = count(music_dir().join("recordings"), &["wav"]);
        vec![
            ("Synths".into(), synths.to_string()),
            ("Drum kits".into(), kits.to_string()),
            ("Samples".into(), view::thousands(samples)),
            ("My recordings".into(), recs.to_string()),
            ("Effects".into(), "reverb, delay, glue…".into()),
        ]
    }
}

/// The song rendered from `from` seconds to a WAV in the runtime folder (on a thread of its own).
fn render(sg: Song, from: f32) -> impl std::future::Future<Output = Result<PathBuf, String>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-beat-render".into()).spawn(move || {
        let s = synth::mix(&sg, None, from);
        let p = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("hearth-beat-maker-play.wav");
        let r = std::fs::write(&p, synth::wav(&s, 2)).map(|_| p).map_err(|e| e.to_string());
        let _ = tx.send(r);
    });
    async move { rx.await.unwrap_or(Err("it stopped".into())) }
}

fn play(a: &mut App) -> Task<Msg> {
    if a.playing.is_some() {
        return Task::none();
    }
    if a.pos >= a.song.length_secs() {
        a.pos = 0.0;
    }
    a.rendering += 1;
    a.busy = "Getting the sound ready…".into();
    let n = a.rendering;
    Task::perform(render(a.song.clone(), a.pos), move |r| Msg::Rendered(n, r))
}

fn start_player(path: &Path) -> Result<(), String> {
    let p = path.display().to_string();
    for cmd in [vec!["pw-play", p.as_str()], vec!["paplay", p.as_str()], vec!["aplay", "-q", p.as_str()]] {
        if on_path(cmd[0]) {
            let c = std::process::Command::new(cmd[0]).args(&cmd[1..]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().map_err(|e| e.to_string())?;
            if let Ok(mut g) = slot(&PLAYER).lock() {
                *g = Some(c);
            }
            return Ok(());
        }
    }
    Err("no sound player here (pw-play, paplay or aplay)".into())
}

fn stop(a: &mut App) {
    stop_child(&PLAYER, false);
    if let Some((t, from)) = a.playing.take() {
        a.pos = (from + t.elapsed().as_secs_f32()).min(a.song.length_secs());
    }
    a.rendering += 1;
    a.busy.clear();
    if let Some((t, at, path)) = a.recording.take() {
        stop_child(&RECORDER, true);
        let secs = t.elapsed().as_secs_f32();
        add_take(a, at, secs, &path);
    }
}

/// A recording onto the Voice-over track (made when there's none), where it started.
fn add_take(a: &mut App, at: f32, secs: f32, path: &Path) {
    if !path.is_file() {
        a.said = "Nothing was recorded (no microphone?)".into();
        return;
    }
    let n = std::fs::read_dir(music_dir().join("recordings")).map(|r| r.count()).unwrap_or(1);
    a.song.patterns.push(Pattern { name: format!("take {n}"), body: Body::Audio { file: path.display().to_string() } });
    let pat = a.song.patterns.len() - 1;
    let ti = match a.song.tracks.iter().position(|t| t.inst.kind == song::Kind::Audio) {
        Some(i) => i,
        None => {
            a.song.tracks.push(Track {
                name: "Voice-over".into(),
                color: 0xc9c1ba,
                inst: song::Inst { kind: song::Kind::Audio, name: "Voice-over".into(), sub: "recordings".into(), mark: "VO".into(), from: 0xc9c1ba, to: 0x6d645d, knobs: [0.5, 0.0, 0.0, 0.2], delay: false, glue: false },
                mute: false,
                solo: false,
                clips: vec![],
            });
            a.song.tracks.len() - 1
        }
    };
    let bars = secs / a.song.sec_per_beat() / a.song.per_bar();
    a.song.tracks[ti].clips.push(Clip { start: a.bar_at(at), len: bars.max(0.25), pat });
    a.save();
}

fn record(a: &mut App) -> Task<Msg> {
    if a.recording.is_some() {
        stop(a);
        return Task::none();
    }
    let dir = music_dir().join("recordings");
    let _ = std::fs::create_dir_all(&dir);
    let n = std::fs::read_dir(&dir).map(|r| r.count()).unwrap_or(0) + 1;
    let path = dir.join(format!("take-{n}.wav"));
    let p = path.display().to_string();
    let cmd: Vec<&str> = if on_path("pw-record") {
        vec!["pw-record", "--rate", "44100", "--channels", "1", "--format", "s16", p.as_str()]
    } else if on_path("arecord") {
        vec!["arecord", "-q", "-f", "S16_LE", "-r", "44100", "-c", "1", p.as_str()]
    } else {
        a.said = "Recording needs PipeWire's pw-record (or arecord).".into();
        return Task::none();
    };
    match std::process::Command::new(cmd[0]).args(&cmd[1..]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn() {
        Ok(c) => {
            if let Ok(mut g) = slot(&RECORDER).lock() {
                *g = Some(c);
            }
            a.recording = Some((Instant::now(), a.pos, path));
            play(a)
        }
        Err(e) => {
            a.said = format!("It couldn't record: {e}");
            Task::none()
        }
    }
}

/// Export for Hearth's videos, on a thread of its own: the files, the beats, the hearth-workspace copy, the site's list.
fn export(sg: Song, e: Export) -> impl std::future::Future<Output = Result<String, String>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-beat-export".into()).spawn(move || {
        let _ = tx.send(export_now(&sg, &e));
    });
    async move { rx.await.unwrap_or(Err("it stopped".into())) }
}

fn mmss(secs: f32) -> String {
    let s = secs.round() as i64;
    format!("{}:{:02}", s / 60, s % 60)
}

fn find_repo(names: &[&str], marker: &str) -> Option<PathBuf> {
    let h = hearth_ui::paths::home();
    let mut tries: Vec<PathBuf> = Vec::new();
    for n in names {
        tries.push(h.join(n));
        tries.push(h.join("Projects").join(n));
        tries.push(h.join("src").join(n));
        tries.push(h.join("GitHub").join(n));
    }
    tries.into_iter().find(|p| p.join(marker).exists())
}

fn export_now(sg: &Song, e: &Export) -> Result<String, String> {
    let name = if e.name.trim().is_empty() { sg.name.clone() } else { e.name.trim().to_string() };
    let sl = slug(&name);
    let dir = music_dir().join("exports").join(&sl);
    std::fs::create_dir_all(&dir).map_err(|x| x.to_string())?;
    let mut done: Vec<String> = Vec::new();
    let mix = synth::mix(sg, None, 0.0);
    let wav = dir.join(format!("{sl}.wav"));
    std::fs::write(&wav, synth::wav(&mix, 2)).map_err(|x| x.to_string())?;
    if e.wav {
        done.push(format!("{sl}.wav"));
    }
    let mp3 = dir.join(format!("{sl}.mp3"));
    if e.mp3 || e.site {
        let o = std::process::Command::new("ffmpeg").args(["-nostdin", "-loglevel", "error", "-y", "-i"]).arg(&wav).args(["-codec:a", "libmp3lame", "-b:a", "256k"]).arg(&mp3).output();
        match o {
            Ok(o) if o.status.success() => done.push(format!("{sl}.mp3")),
            _ => done.push(".mp3 skipped (it needs ffmpeg)".into()),
        }
    }
    if e.stems {
        let st = dir.join("stems");
        std::fs::create_dir_all(&st).map_err(|x| x.to_string())?;
        for (i, t) in sg.tracks.iter().enumerate() {
            let s = synth::mix(sg, Some(i), 0.0);
            std::fs::write(st.join(format!("{:02}-{}.wav", i + 1, slug(&t.name))), synth::wav(&s, 2)).map_err(|x| x.to_string())?;
        }
        done.push(format!("{} stems", sg.tracks.len()));
    }
    // the song for LMMS, and Hearth's own file
    std::fs::write(dir.join(format!("{sl}.mmp")), song::to_mmp(sg)).map_err(|x| x.to_string())?;
    let src = serde_json::to_string_pretty(&song::to_json(sg)).unwrap_or_default();
    std::fs::write(dir.join(format!("{sl}.hbm.json")), &src).map_err(|x| x.to_string())?;
    let beats = beat_marks(sg);
    if e.beats {
        std::fs::write(dir.join(format!("{sl}-beats.json")), serde_json::to_string_pretty(&beats).unwrap_or_default()).map_err(|x| x.to_string())?;
        done.push("the beats for video cuts".into());
    }
    if !e.wav {
        let _ = std::fs::remove_file(&wav);
    }
    // hearth-workspace: the song's source (its own rules: renders aren't kept there, they can be made again)
    let mut ws_note = String::new();
    if let Some(ws) = find_repo(&["hearth-workspace"], "INDEX.md") {
        let day = {
            let t = hearth_ui::time::local(hearth_ui::time::now());
            format!("{:04}-{:02}-{:02}", t.year, t.mon + 1, t.day)
        };
        let item = format!("{day}-{sl}-music");
        let d = ws.join("hearth/assets").join(&item);
        std::fs::create_dir_all(&d).map_err(|x| x.to_string())?;
        std::fs::write(d.join("song.hbm.json"), &src).map_err(|x| x.to_string())?;
        std::fs::write(d.join(format!("{sl}.mmp")), song::to_mmp(sg)).map_err(|x| x.to_string())?;
        if e.beats {
            std::fs::write(d.join("beats.json"), serde_json::to_string_pretty(&beats).unwrap_or_default()).map_err(|x| x.to_string())?;
        }
        let readme = format!(
            "# {name}\n\nA song made in Hearth Beat Maker ({} · {} BPM · {} · {} bars). Status: draft.\nBelongs to: Hearth's videos (its music list on hearthos.fun).\n\nRebuild: open song.hbm.json in Hearth Beat Maker and Export (or {sl}.mmp in LMMS: `lmms render {sl}.mmp`).\nThe .wav and .mp3 aren't kept here (they can be made again); the site's copy is in hearth-os site/music.\n",
            mmss(sg.length_secs()),
            sg.bpm.round(),
            sg.key_name(),
            sg.bars
        );
        std::fs::write(d.join("README.md"), readme).map_err(|x| x.to_string())?;
        let idx = ws.join("INDEX.md");
        if let Ok(t) = std::fs::read_to_string(&idx) {
            let line = format!("- `hearth/assets/{item}`: {name}, a song made in Hearth Beat Maker ({}, {} BPM, {}). Status: draft.", mmss(sg.length_secs()), sg.bpm.round(), sg.key_name());
            if !t.contains(&format!("hearth/assets/{item}`")) {
                let new = match t.find("## Hearth OS\n") {
                    Some(i) => format!("{}{}\n{}", &t[..i + "## Hearth OS\n".len()], line, &t[i + "## Hearth OS\n".len()..]),
                    None => format!("{t}\n{line}\n"),
                };
                let _ = settings_file::write_text(&idx, &new);
            }
        }
        ws_note = format!("Saved to hearth-workspace: hearth/assets/{item} (push it there yourself)");
    }
    // hearthos.fun's music list: your hearth-os folder's site/index.html, never committed for you
    let mut site_note = String::new();
    if e.site {
        match find_repo(&["hearth-os", "nimbus"], "site/index.html") {
            Some(repo) if mp3.is_file() => {
                let file = format!("hearth-{sl}");
                let _ = std::fs::copy(&mp3, repo.join("site/music").join(format!("{file}.mp3")));
                let idx = repo.join("site/index.html");
                if let Ok(t) = std::fs::read_to_string(&idx) {
                    if !t.contains(&format!("'{file}']")) {
                        if let Some(i) = t.find("var TRACKS = [\n") {
                            let at = i + "var TRACKS = [\n".len();
                            let note = if sg.note.is_empty() { "Hearth's music".to_string() } else { sg.note.clone() };
                            let line = format!("    ['{}', '{}', '{}', '{file}'],\n", name.replace('\'', "\\'"), note.replace('\'', "\\'"), mmss(sg.length_secs()));
                            let _ = settings_file::write_text(&idx, &format!("{}{}{}", &t[..at], line, &t[at..]));
                        }
                    }
                }
                site_note = format!("Added to hearthos.fun's music list in {} (commit it with the site)", repo.display());
            }
            Some(_) => site_note = "Not added to the music list: it needs the .mp3 (ffmpeg)".into(),
            None => site_note = "Not added to the music list: no hearth-os folder found here".into(),
        }
    }
    let mut out = format!("In {}: {}.", dir.display(), done.join(", "));
    for n in [ws_note, site_note] {
        if !n.is_empty() {
            out.push_str(&format!("\n{n}."));
        }
    }
    Ok(out)
}

/// The beats for video cuts: every beat and bar's time, and the big moments (big hits and the effects' hits).
fn beat_marks(sg: &Song) -> Value {
    let spb = sg.sec_per_beat();
    let per_bar = sg.per_bar();
    let beats: Vec<f32> = (0..(sg.bars as f32 * per_bar) as usize).map(|b| (b as f32 * spb * 1000.0).round() / 1000.0).collect();
    let bars: Vec<f32> = (0..sg.bars).map(|b| (b as f32 * per_bar * spb * 1000.0).round() / 1000.0).collect();
    let mut hits: Vec<f32> = Vec::new();
    for t in &sg.tracks {
        for c in &t.clips {
            let Some(p) = sg.patterns.get(c.pat) else { continue };
            let starts_big = match &p.body {
                Body::Beat { rows, .. } => rows.iter().any(|r| r.drum == song::Drum::BigHit && r.hits.first().copied().unwrap_or(0) > 0),
                Body::Notes(_) => t.inst.kind == song::Kind::Fx && p.name.to_lowercase().contains("hit"),
                _ => false,
            };
            if starts_big {
                hits.push((c.start * per_bar * spb * 1000.0).round() / 1000.0);
            }
        }
    }
    hits.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    hits.dedup();
    json!({"song": sg.name, "bpm": sg.bpm, "seconds": sg.length_secs(), "beats": beats, "bars": bars, "hits": hits})
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Describe(dm) => {
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(view::block_action) {
                return update(a, m);
            }
            return a.describe.update(dm).map(Msg::Describe);
        }
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Play => {
            if a.playing.is_some() {
                stop(a);
                return Task::none();
            }
            return play(a);
        }
        Msg::Stop => stop(a),
        Msg::ToStart => {
            let was = a.playing.is_some();
            stop(a);
            a.pos = 0.0;
            if was {
                return play(a);
            }
        }
        Msg::Record => return record(a),
        Msg::Rendered(n, r) => {
            if n != a.rendering {
                return Task::none();
            }
            a.busy.clear();
            match r.and_then(|p| start_player(&p)) {
                Ok(()) => a.playing = Some((Instant::now(), a.pos)),
                Err(e) => a.said = e,
            }
        }
        Msg::Frame => {
            if let Some((t, from)) = a.playing {
                a.pos = from + t.elapsed().as_secs_f32();
                if a.pos >= a.song.length_secs() {
                    stop(a);
                    a.pos = a.song.length_secs();
                }
            }
        }
        Msg::Cpu => a.cpu_tick(),
        Msg::Seek(bar) => {
            let was = a.playing.is_some();
            stop(a);
            a.pos = (bar.max(0.0) * a.song.per_bar() * a.song.sec_per_beat()).min(a.song.length_secs());
            if was {
                return play(a);
            }
        }
        Msg::Bpm(d) => {
            a.song.bpm = (a.song.bpm + d as f32).clamp(40.0, 240.0);
            a.save();
        }
        Msg::Sig => {
            a.song.sig = match a.song.sig {
                (4, 4) => (3, 4),
                (3, 4) => (6, 8),
                _ => (4, 4),
            };
            a.save();
        }
        Msg::Key(k, minor) => {
            a.song.key = k % 12;
            a.song.minor = minor;
            a.dlg = None;
            a.save();
        }
        Msg::Mode(md) => a.mode = md,
        Msg::PickTrack(i) => a.sel_track = i.min(a.song.tracks.len().saturating_sub(1)),
        Msg::Mute(i) => {
            if let Some(t) = a.song.tracks.get_mut(i) {
                t.mute = !t.mute;
            }
            a.save();
        }
        Msg::Solo(i) => {
            if let Some(t) = a.song.tracks.get_mut(i) {
                t.solo = !t.solo;
            }
            a.save();
        }
        Msg::Lane(ti, bar) => {
            a.sel_track = ti;
            let Some(t) = a.song.tracks.get(ti) else { return Task::none() };
            let hit = t.clips.iter().position(|c| bar >= c.start && bar < c.start + c.len);
            match (a.mode, hit) {
                (Mode::Pick, Some(ci)) | (Mode::Draw, Some(ci)) => {
                    a.sel_clip = Some((ti, ci));
                    let pat = t.clips[ci].pat;
                    match a.song.patterns[pat].body {
                        Body::Beat { .. } => a.beat_pat = Some(pat),
                        Body::Notes(_) => {
                            a.notes_pat = Some(pat);
                            a.notes_track = ti;
                            a.sel_notes.clear();
                        }
                        _ => {}
                    }
                }
                (Mode::Draw, None) => {
                    // a new clip of two bars: the track's last pattern again (or a new one of its kind)
                    let start = (bar / 2.0).floor() * 2.0;
                    let pat = match t.clips.last().map(|c| c.pat) {
                        Some(p) => p,
                        None => {
                            let body = if t.inst.kind == song::Kind::Drums { Body::Beat { rows: vec![Row { name: "Kick".into(), drum: song::Drum::Kick, hits: vec![0; 16] }], steps: 16, swing: 0.0 } } else { Body::Notes(vec![]) };
                            a.song.patterns.push(Pattern { name: format!("{} {}", t.name, a.song.patterns.len() + 1), body });
                            a.song.patterns.len() - 1
                        }
                    };
                    a.song.tracks[ti].clips.push(Clip { start, len: 2.0, pat });
                    a.save();
                }
                (Mode::Cut, Some(ci)) => {
                    let c = a.song.tracks[ti].clips[ci].clone();
                    let at = bar.round();
                    if at > c.start && at < c.start + c.len {
                        a.song.tracks[ti].clips[ci].len = at - c.start;
                        a.song.tracks[ti].clips.push(Clip { start: at, len: c.start + c.len - at, pat: c.pat });
                        a.save();
                    }
                }
                _ => {}
            }
        }
        Msg::MoveClip(ti, ci, start) => {
            if let Some(c) = a.song.tracks.get_mut(ti).and_then(|t| t.clips.get_mut(ci)) {
                let bars = a.song.bars as f32;
                c.start = start.round().clamp(0.0, (bars - c.len).max(0.0));
                a.save();
            }
        }
        Msg::Step(r, i) => {
            if let Some((rows, _, _)) = a.beat_rows_mut() {
                if let Some(h) = rows.get_mut(r).and_then(|row| row.hits.get_mut(i)) {
                    *h = if *h > 0 { 0 } else { 100 };
                }
                a.save();
            }
        }
        Msg::StepVel(r, i, v) => {
            if let Some((rows, _, _)) = a.beat_rows_mut() {
                if let Some(h) = rows.get_mut(r).and_then(|row| row.hits.get_mut(i)) {
                    if *h > 0 {
                        *h = v.clamp(1, 127);
                    }
                }
                a.save();
            }
        }
        Msg::Swing => {
            if let Some((_, _, sw)) = a.beat_rows_mut() {
                *sw = match *sw as i32 {
                    0 => 12.0,
                    12 => 25.0,
                    25 => 33.0,
                    33 => 50.0,
                    _ => 0.0,
                };
                a.save();
            }
        }
        Msg::AddRow => {
            if let Some((rows, steps, _)) = a.beat_rows_mut() {
                let have: Vec<song::Drum> = rows.iter().map(|r| r.drum).collect();
                if let Some((d, n)) = song::DRUMS.iter().find(|(d, _)| !have.contains(d)) {
                    rows.push(Row { name: n.to_string(), drum: *d, hits: vec![0; *steps] });
                }
                a.save();
            }
        }
        Msg::Grid(g) => a.grid = g,
        Msg::Fit => {
            a.fit = !a.fit;
            if a.fit {
                let sg = a.song.clone();
                if let Some(n) = a.notes_mut() {
                    for x in n.iter_mut() {
                        x.key = sg.fit(x.key);
                    }
                }
                a.save();
            }
        }
        Msg::AddNote(key, beat) => {
            let key = if a.fit { a.song.fit(key) } else { key };
            let g = a.grid;
            let start = (beat / g).floor() * g;
            if let Some(n) = a.notes_mut() {
                n.push(Note { key, start, len: g * 2.0, vel: 100 });
                let i = n.len() - 1;
                a.sel_notes = vec![i];
                a.save();
            }
        }
        Msg::PickNote(i, add) => {
            if add {
                if let Some(k) = a.sel_notes.iter().position(|x| *x == i) {
                    a.sel_notes.remove(k);
                } else {
                    a.sel_notes.push(i);
                }
            } else {
                a.sel_notes = vec![i];
            }
        }
        Msg::MoveNote(i, key, start) => {
            let key = if a.fit { a.song.fit(key) } else { key };
            let g = a.grid;
            if let Some(x) = a.notes_mut().and_then(|n| n.get_mut(i)) {
                x.key = key.clamp(24, 108);
                x.start = ((start / g).round() * g).max(0.0);
            }
            a.save();
        }
        Msg::SizeNote(i, len) => {
            let g = a.grid;
            if let Some(x) = a.notes_mut().and_then(|n| n.get_mut(i)) {
                x.len = ((len / g).round() * g).max(g);
            }
            a.save();
        }
        Msg::DelNote(i) => {
            if let Some(n) = a.notes_mut() {
                if i < n.len() {
                    n.remove(i);
                }
            }
            a.sel_notes.clear();
            a.save();
        }
        Msg::NoteVel(i, v) => {
            if let Some(x) = a.notes_mut().and_then(|n| n.get_mut(i)) {
                x.vel = v.clamp(1, 127);
            }
            a.save();
        }
        Msg::RollScroll(d) => a.roll_top = (a.roll_top + d).clamp(36, 108),
        Msg::MakeChords(kind) => {
            // each picked note (or every note) becomes a chord of the key: its third and fifth (and seventh) in the key
            let sg = a.song.clone();
            let picked = a.sel_notes.clone();
            if let Some(n) = a.notes_mut() {
                let idx: Vec<usize> = if picked.is_empty() { (0..n.len()).collect() } else { picked.clone() };
                let sc = sg.scale();
                let up = |key: i32, steps: usize| -> i32 {
                    let mut k = key;
                    let mut left = steps;
                    while left > 0 {
                        k += 1;
                        if sc.contains(&k.rem_euclid(12)) {
                            left -= 1;
                        }
                    }
                    k
                };
                let mut add = Vec::new();
                for i in idx {
                    let Some(x) = n.get(i).cloned() else { continue };
                    let root = sg.fit(x.key);
                    let mut keys = vec![up(root, 2), up(root, 4)];
                    if kind == "seventh" {
                        keys.push(up(root, 6));
                    }
                    for k in keys {
                        if !n.iter().any(|y| y.key == k && (y.start - x.start).abs() < 0.01) {
                            add.push(Note { key: k, start: x.start, len: x.len, vel: x.vel });
                        }
                    }
                }
                n.extend(add);
                a.dlg = None;
                a.save();
            }
        }
        Msg::Knob(k, v) => {
            let i = a.sel_track;
            if let Some(t) = a.song.tracks.get_mut(i) {
                t.inst.knobs[k] = v.clamp(0.0, 1.0);
            }
            a.save();
        }
        Msg::Find(s) => {
            a.find = s;
            if !a.find.is_empty() && !matches!(a.dlg, Some(Dlg::Library(_))) {
                a.dlg = Some(Dlg::Library("Find".into()));
            }
        }
        Msg::Open(d) => a.dlg = Some(d),
        Msg::Close => {
            a.dlg = None;
            a.export = None;
        }
        Msg::AddInst(i) => {
            let p = song::presets();
            if let Some(inst) = p.get(i).cloned() {
                let body = if inst.kind == song::Kind::Drums {
                    Body::Beat { rows: ["Kick", "Snare", "Closed hat"].iter().map(|n| Row { name: n.to_string(), drum: song::drum_of(n), hits: vec![0; 16] }).collect(), steps: 16, swing: 0.0 }
                } else {
                    Body::Notes(vec![])
                };
                a.song.patterns.push(Pattern { name: format!("{} 1", inst.name), body });
                let pat = a.song.patterns.len() - 1;
                a.song.tracks.push(Track { name: inst.name.clone(), color: inst.from, inst, mute: false, solo: false, clips: vec![Clip { start: 0.0, len: 2.0, pat }] });
                a.sel_track = a.song.tracks.len() - 1;
                a.dlg = None;
                a.save();
            }
        }
        Msg::AddSample(file) => {
            // LMMS's samples are mostly Ogg: made a WAV first (ffmpeg), kept with your recordings
            let file = if file.to_lowercase().ends_with(".wav") {
                file
            } else {
                let dir = music_dir().join("samples");
                let _ = std::fs::create_dir_all(&dir);
                let out = dir.join(format!("{}.wav", Path::new(&file).file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_default()));
                let ok = std::process::Command::new("ffmpeg").args(["-nostdin", "-loglevel", "error", "-y", "-i", &file, "-ar", "44100", "-ac", "1", "-c:a", "pcm_s16le"]).arg(&out).status().map(|s| s.success()).unwrap_or(false);
                if !ok {
                    a.said = "That sound needs ffmpeg to be added (Hearth Store › ffmpeg).".into();
                    return Task::none();
                }
                out.display().to_string()
            };
            let w = synth::read_wav(&file);
            let secs = w.len() as f32 / synth::RATE as f32;
            if secs <= 0.0 {
                a.said = "Only 16-bit WAV sounds can be added for now.".into();
            } else {
                let at = a.pos;
                a.dlg = None;
                add_take(a, at, secs, Path::new(&file));
            }
        }
        Msg::Effect(e) => {
            let i = a.sel_track;
            if let Some(t) = a.song.tracks.get_mut(i) {
                match e {
                    "delay" => t.inst.delay = !t.inst.delay,
                    "glue" => t.inst.glue = !t.inst.glue,
                    _ => t.inst.knobs[3] = if t.inst.knobs[3] > 0.05 { 0.0 } else { 0.45 },
                }
            }
            a.save();
        }
        Msg::ExportOpen => a.export = Some(a.new_export()),
        Msg::ExportName(s) => {
            if let Some(e) = a.export.as_mut() {
                e.name = s;
            }
        }
        Msg::ExportToggle(k) => {
            if let Some(e) = a.export.as_mut() {
                match k {
                    "wav" => e.wav = !e.wav,
                    "mp3" => e.mp3 = !e.mp3,
                    "stems" => e.stems = !e.stems,
                    "site" => e.site = !e.site,
                    _ => e.beats = !e.beats,
                }
            }
        }
        Msg::ExportGo => {
            if let Some(e) = a.export.take() {
                a.busy = "Exporting…".into();
                return Task::perform(export(a.song.clone(), e), Msg::Exported);
            }
        }
        Msg::Exported(r) => {
            a.busy.clear();
            a.dlg = Some(match r {
                Ok(s) => Dlg::Said("Exported".into(), s),
                Err(e) => Dlg::Said("It couldn't be exported".into(), e),
            });
        }
        Msg::OpenLmms => {
            let Some(cmd) = lmms_cmd() else {
                a.dlg = Some(Dlg::NoLmms);
                return Task::none();
            };
            let mmp = a.file.with_extension("").with_extension("mmp");
            if let Err(e) = std::fs::write(&mmp, song::to_mmp(&a.song)) {
                a.said = format!("The LMMS project couldn't be written: {e}");
                return Task::none();
            }
            let mut c: Vec<String> = cmd;
            c.push(mmp.display().to_string());
            let refs: Vec<&str> = c.iter().map(|s| s.as_str()).collect();
            run::detached(&refs);
            a.said = format!("Opened in LMMS: {}", mmp.display());
        }
        Msg::InstallLmms => {
            a.dlg = None;
            a.busy = "Installing LMMS from Flathub (for you only)…".into();
            return Task::perform(
                run::spawn(run::args(&["sh", "-c", "flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo && flatpak install --user -y --noninteractive flathub io.lmms.LMMS"])),
                Msg::Installed,
            );
        }
        Msg::Installed(o) => {
            a.busy.clear();
            a.said = if o.ok { "LMMS is installed.".into() } else { format!("LMMS couldn't be installed: {}", o.why()) };
        }
        Msg::LmmsRendered(o) => {
            a.busy.clear();
            a.said = if o.ok { "LMMS rendered it.".into() } else { format!("LMMS couldn't render it: {}", o.why()) };
        }
        Msg::KeyEv(ev) => {
            if let KEvent::KeyPressed { key, modifiers, .. } = ev {
                if let Some(dm) = hearth_ui::arrange::arrange_key(&a.describe, &key, modifiers) {
                    return update(a, Msg::Describe(dm));
                }
                match key.as_ref() {
                    Key::Named(Named::Space) => return update(a, Msg::Play),
                    Key::Named(Named::Home) => return update(a, Msg::ToStart),
                    Key::Named(Named::Escape) => {
                        a.dlg = None;
                        a.export = None;
                    }
                    Key::Named(Named::Delete) => {
                        let mut s = a.sel_notes.clone();
                        s.sort_unstable_by(|x, y| y.cmp(x));
                        if let Some(n) = a.notes_mut() {
                            for i in s {
                                if i < n.len() {
                                    n.remove(i);
                                }
                            }
                        }
                        a.sel_notes.clear();
                        a.save();
                    }
                    Key::Character("r") if !modifiers.control() => return update(a, Msg::Record),
                    _ => {}
                }
            }
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::Quit => {
            stop(a);
            return win::close();
        }
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

fn subscription(a: &App) -> Subscription<Msg> {
    // keys no text field took (so a space typed into a name doesn't play the song)
    let keys = iced::event::listen_with(|ev, status, _id| match (ev, status) {
        (iced::Event::Keyboard(k), iced::event::Status::Ignored) => Some(Msg::KeyEv(k)),
        _ => None,
    });
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme), keys];
    // only while playing: the playhead 30 times a second, the CPU once a second
    if a.playing.is_some() || a.recording.is_some() {
        s.push(iced::time::every(std::time::Duration::from_millis(33)).map(|_| Msg::Frame));
        s.push(iced::time::every(std::time::Duration::from_secs(1)).map(|_| Msg::Cpu));
    }
    Subscription::batch(s)
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view::view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::handle_cli(
        "hearth-beat-maker-app",
        "Hearth Beat Maker: songs from patterns, a piano roll and instruments (LMMS).\n  --open FILE.hbm.json   --at SECONDS   --track NAME   --notes PATTERN   --beat PATTERN   --select-notes 9,10,11   --show export|library\n  --render FILE.hbm.json OUT.wav   render a song with no window",
    ) {
        return Ok(());
    }
    if let Some(f) = app::arg("--render") {
        let out = std::env::args().skip_while(|x| x != "--render").nth(2).unwrap_or_else(|| "out.wav".into());
        let sg = settings_file::read(Path::new(&f)).map().and_then(|m| song::from_json(&Value::Object(m)));
        let code = match sg {
            Some(sg) => match std::fs::write(&out, synth::wav(&synth::mix(&sg, None, 0.0), 2)) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            },
            None => {
                eprintln!("{f} isn't a Hearth Beat Maker song");
                2
            }
        };
        std::process::exit(code);
    }
    iced::application(App::new, update, shown)
        .title(|a: &App| format!("Hearth Beat Maker · {}", a.song.name))
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(app::window("hearth-beat-maker", (1600.0, 900.0), (1100.0, 680.0)))
        .default_font(app::INTER)
        .run()
}
