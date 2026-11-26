//! hearth-listen-app: Hearth Listen (docs/mockups/checklists/open-source-07-listen.md; the approved board
//! docs/mockups/open-source-apps/07-listen.png). Speech to words on your own PC with whisper.cpp (MIT): your
//! recordings, what was said in each with its times and voices, live captions for anything playing, and Send to
//! Voice Studio (Hearth's own, on the home server: the words come back with their times, so it can cut takes by what
//! was said and make subtitles for both video sizes).
//!
//! Nothing is sent anywhere: whisper.cpp runs here (on the Nvidia card when there is one), and its models are in
//! ~/.local/share/hearth/listen. Your choices: ~/.config/nimbus/listen.json (prefs: only what you change).
//!
//!   --open ID     that recording       --at SECS   the player there       --search WORDS   Search every word
//!        --show export|model|lang|runs   a menu open
//!   --captions    the live captions bar (its own small window, on top; Listen starts it when you turn captions on)
//! $HEARTH_LISTEN_DATA: another folder instead of ~/.local/share/hearth/listen (CI's pictures use a made-up one).

mod captions;
mod recs;
mod whisper;

use hearth_oss::prefs::Prefs;
use hearth_oss::w::{self, Sz};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::ui::{self, t, tb, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::{canvas, column, container, mouse_area, row, scrollable, slider, Column, Space};
use iced::{mouse, Color, Element, Length, Padding, Subscription, Task};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

const DEFAULT_WORDS: &[&str] = &["Hearth", "Mantel", "Ithil", "Ember", "Firehold", "Anatase", "Peppermint", "Beacon", "Tributary", "Gravesleep", "Noclip Protocol", "Dragongamer9", "Tailscale", "Vaultwarden", "Nextcloud"];
const VS_PORT: u16 = 8890;

#[derive(Debug, Clone)]
enum Msg {
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Tick,
    Poll,
    Loaded(Vec<recs::Rec>),
    Search(String),
    Pick(String),
    Record,
    OpenFile,
    Picked(String),
    Made(String, Result<String, String>),
    Copy,
    Export(&'static str),
    Exported(Result<String, String>),
    SendVs,
    Sent(Result<String, String>),
    Play,
    Seek(f64),
    SeekFrac(f32),
    Speed(f32),
    Menu(&'static str),
    Model(String),
    RunsOn(bool),
    Lang(String),
    Toggle(&'static str),
    EditWords,
    Words(String),
    WordsDone,
    Captions,
    CapWhere(&'static str),
    CapSize(f32),
    CapSizeDone,
    CapListen(&'static str),
    Install,
    GetModel(String),
    Got(String, run::Output),
    Again,
    Gpu(String),
    MakeWords(String),
    Drag,
    Max,
    Min,
    Close,
    Resize(iced::window::Direction),
}

struct Play {
    child: Option<Child>,
    started: Instant,
    from: f64,
}

struct Recording {
    child: Child,
    started: Instant,
    dir: PathBuf,
}

struct App {
    describe: hearth_ui::describe::Describe,
    p: Pal,
    prefs: Prefs,
    recs: Vec<recs::Rec>,
    loaded: bool,
    open: String,
    q: String,
    cli: Option<PathBuf>,
    gpu: String,
    install: Value,
    menu: Option<&'static str>,
    words_edit: Option<String>,
    play: Option<Play>,
    pos: f64,
    recording: Option<Recording>,
    said: String,
    getting: String,
    cap_size: f32,
    models_size: u64,
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let prefs = Prefs::load("listen");
        let cap_size = prefs.f("captionSize", 28.0) as f32;
        let a = App {
            describe: hearth_ui::describe::Describe::new("listen", "Hearth Listen").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            prefs,
            recs: Vec::new(),
            loaded: false,
            open: app::arg("--open").unwrap_or_default(),
            q: app::arg("--search").unwrap_or_default(),
            cli: whisper::cli(),
            gpu: String::new(),
            install: whisper::install_state(),
            menu: match app::arg("--show").as_deref() {
                Some("export") => Some("export"),
                Some("model") => Some("model"),
                Some("lang") => Some("lang"),
                Some("runs") => Some("runs"),
                _ => None,
            },
            words_edit: None,
            play: None,
            pos: app::arg("--at").and_then(|x| x.parse().ok()).unwrap_or(0.0),
            recording: None,
            said: String::new(),
            getting: String::new(),
            cap_size,
            models_size: hearth_oss::dir_size(&whisper::models_dir()),
        };
        (a, Task::batch([load(), Task::perform(bg(whisper::gpu), |g| Msg::Gpu(g.unwrap_or_default()))]))
    }

    fn rec(&self) -> Option<&recs::Rec> {
        self.recs.iter().find(|r| r.id == self.open).or(self.recs.first())
    }
    fn how(&self) -> whisper::How {
        whisper::How {
            model: self.model(),
            gpu: self.prefs.b("gpu", true) && !self.gpu.is_empty(),
            lang: self.prefs.s("language", "auto"),
            voices: self.prefs.b("voices", true),
            punct: self.prefs.b("punctuation", true),
            translate: self.prefs.b("translate", false),
            words: self.words(),
        }
    }
    fn model(&self) -> String {
        self.prefs.s("model", "large-v3-turbo")
    }
    fn words(&self) -> Vec<String> {
        if self.prefs.get("words").is_some() {
            self.prefs.list("words")
        } else {
            DEFAULT_WORDS.iter().map(|s| s.to_string()).collect()
        }
    }
    fn playing(&self) -> bool {
        self.play.as_ref().map(|p| p.child.is_some()).unwrap_or(false)
    }
    fn speed(&self) -> f32 {
        self.prefs.f("speed", 1.0) as f32
    }
    fn set(&mut self, k: &str, v: Value) {
        if let Err(e) = self.prefs.set(k, v) {
            self.said = e;
        }
    }
}

fn load() -> Task<Msg> {
    Task::perform(bg(recs::all), |v| Msg::Loaded(v.unwrap_or_default()))
}

/// Work off the window's thread; ready when it's done.
fn bg<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> impl std::future::Future<Output = Option<T>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-listen".into()).spawn(move || {
        let _ = tx.send(f());
    });
    async move { rx.await.ok() }
}

/// A recording's sound into words with whisper.cpp (and, for Tell voices apart, its tinydiarize pass for where the
/// voice changes). Its words.json is written whole when it's done.
fn make_words(dir: PathBuf, how: whisper::How, cli: PathBuf) -> Result<String, String> {
    let wav = dir.join("audio.wav");
    if !wav.is_file() {
        return Err("There's no sound in this recording".into());
    }
    if !whisper::have_model(&how.model) {
        return Err(format!("Get the {} model first (How it listens › Model)", whisper::model_name(&how.model)));
    }
    let started = Instant::now();
    let out = dir.join("whisper");
    let o = run::blocking(&whisper::args(&cli, &how, &wav, &out, &how.model, false));
    if !o.ok {
        return Err(format!("whisper.cpp stopped: {}", o.why()));
    }
    let j: Value = std::fs::read(dir.join("whisper.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).ok_or("whisper.cpp wrote no words")?;
    let (mut segs, lang, _) = whisper::parse(&j);
    if how.voices {
        if whisper::have_model(whisper::TDRZ.0) {
            let o2 = run::blocking(&whisper::args(&cli, &how, &wav, &dir.join("turns"), whisper::TDRZ.0, true));
            if o2.ok {
                if let Some(j2) = std::fs::read(dir.join("turns.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) {
                    let (_, _, turns) = whisper::parse(&j2);
                    whisper::voices_from_turns(&mut segs, &turns);
                }
            }
            let _ = std::fs::remove_file(dir.join("turns.json"));
        }
    }
    if !how.punct {
        for s in segs.iter_mut() {
            s.text = whisper::plain(&s.text);
        }
    }
    let secs = whisper::read_wav(&wav).map(|w| w.secs()).unwrap_or(0.0);
    let took = started.elapsed().as_secs_f64();
    recs::write(&dir.join("words.json"), &whisper::to_json(&segs, &lang, secs, took.round(), &how.model))?;
    let _ = std::fs::remove_file(dir.join("whisper.json"));
    Ok(format!("Made into words in {} s", took.round()))
}

/// Any sound or video file as 16 kHz mono WAV (ffmpeg), or a copy when it already is one.
fn to_wav(src: &str, dst: &std::path::Path) -> Result<(), String> {
    if let Some(w) = whisper::read_wav(std::path::Path::new(src)) {
        if w.rate == 16000 && w.ch == 1 {
            return std::fs::copy(src, dst).map(|_| ()).map_err(|e| e.to_string());
        }
    }
    let o = run::blocking(&run::args(&["ffmpeg", "-nostdin", "-y", "-loglevel", "error", "-i", src, "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le", &dst.to_string_lossy()]));
    if o.ok {
        Ok(())
    } else if o.err.contains("No such file") && o.code == -1 {
        Err("Reading that kind of file needs ffmpeg (Hearth Store › Developer tools), or open a 16 kHz WAV".into())
    } else {
        Err(format!("That file couldn't be read: {}", o.why()))
    }
}

fn stop_play(a: &mut App) {
    if let Some(pl) = a.play.as_mut() {
        if let Some(mut c) = pl.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

fn start_play(a: &mut App) {
    stop_play(a);
    let Some(r) = a.rec().cloned() else { return };
    let Some(wav) = whisper::read_wav(&r.audio()) else {
        a.said = "This recording has no sound to play.".into();
        return;
    };
    if a.pos >= r.secs - 0.2 {
        a.pos = 0.0;
    }
    let bytes = whisper::slice(&wav, a.pos, a.speed());
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("hearth-listen");
    let _ = std::fs::create_dir_all(&dir);
    let f = dir.join("play.wav");
    if std::fs::write(&f, bytes).is_err() {
        a.said = "The sound couldn't be got ready to play.".into();
        return;
    }
    let mut child = None;
    for p in ["pw-play", "paplay", "aplay"] {
        if let Ok(c) = Command::new(p).arg(&f).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
            child = Some(c);
            break;
        }
    }
    if child.is_none() {
        a.said = "There's nothing here to play sound with (pw-play).".into();
        return;
    }
    a.play = Some(Play { child, started: Instant::now(), from: a.pos });
}

fn start_record(a: &mut App) {
    let dir = match recs::create(&format!("Recording, {}", hearth_ui::time::when(hearth_oss::now())), "Hearth Listen") {
        Ok(d) => d,
        Err(e) => {
            a.said = e;
            return;
        }
    };
    let f = dir.join("audio.wav");
    let fs = f.to_string_lossy().to_string();
    let tries: [Vec<&str>; 2] = [vec!["pw-record", "--rate", "16000", "--channels", "1", "--format", "s16", &fs], vec!["parecord", "--rate=16000", "--channels=1", "--format=s16le", "--file-format=wav", &fs]];
    for c in tries.iter() {
        if let Ok(ch) = Command::new(c[0]).args(&c[1..]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
            a.recording = Some(Recording { child: ch, started: Instant::now(), dir });
            a.said = "Recording from your microphone…".into();
            return;
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    a.said = "There's nothing here to record with (pw-record).".into();
}

fn stop_record(a: &mut App) -> Task<Msg> {
    let Some(mut r) = a.recording.take() else { return Task::none() };
    // SIGINT: pw-record finishes the WAV's header before it ends
    unsafe {
        libc::kill(r.child.id() as i32, libc::SIGINT);
    }
    let _ = r.child.wait();
    let id = r.dir.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
    a.open = id.clone();
    words_task(a, id, r.dir)
}

fn words_task(a: &mut App, id: String, dir: PathBuf) -> Task<Msg> {
    let Some(cli) = a.cli.clone() else {
        a.said = "Install whisper.cpp first (How it listens).".into();
        return load();
    };
    let how = a.how();
    if let Some(r) = a.recs.iter_mut().find(|r| r.id == id) {
        r.busy = "Making it into words…".into();
    }
    a.said = format!("Making it into words with whisper.cpp ({})…", whisper::model_name(&how.model));
    Task::batch([load(), Task::perform(bg(move || make_words(dir, how, cli)), move |r| Msg::Made(id.clone(), r.unwrap_or_else(|| Err("it stopped".into()))))])
}

/// Send to Voice Studio: its API key from its own command (never shown, never kept), then the words with their times.
fn send_vs(r: recs::Rec) -> Result<String, String> {
    let o = run::blocking(&run::args(&["python3", &run::shell_helper("hearth-voice-studio"), "key-show"]));
    let key = o.json().and_then(|j| j.get("key").and_then(Value::as_str).map(str::to_string)).filter(|k| !k.is_empty()).ok_or("Voice Studio isn't set up on this PC (Settings › Home server › Voice Studio)")?;
    let segs = whisper::to_json(&r.segs, &r.lang, r.secs, r.took, &r.model);
    let body = json!({"title": r.title, "segments": segs["segments"], "language": r.lang, "duration": r.secs, "source": r.source});
    let resp = hearth_oss::http::send(&hearth_oss::http::Req::new("POST", &format!("http://127.0.0.1:{VS_PORT}/api/jobs/words")).bearer(&key).json(&body).timeout(20)).map_err(|_| "Voice Studio isn't on: turn it on in Settings › Home server › Voice Studio".to_string())?;
    if resp.ok() {
        Ok(format!("Sent to Voice Studio: \"{}\" is in its transcripts, with text, SRT and VTT.", r.title))
    } else {
        Err(format!("Voice Studio said no: {}", resp.json().and_then(|j| j.get("error").and_then(Value::as_str).map(str::to_string)).unwrap_or_else(|| resp.status.to_string())))
    }
}

fn export_file(r: &recs::Rec, ext: &str) -> Result<String, String> {
    let body = match ext {
        "srt" => whisper::to_srt(&r.segs),
        "vtt" => whisper::to_vtt(&r.segs),
        _ => whisper::to_txt(&r.segs),
    };
    let safe: String = r.title.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == ',' || c == '.' { c } else { '_' }).collect();
    let docs = hearth_ui::paths::home().join("Documents");
    let start = docs.join(format!("{}.{ext}", safe.trim()));
    let picked = run::blocking(&run::args(&["kdialog", "--getsavefilename", &start.to_string_lossy(), &format!("*.{ext}")]));
    let path = if picked.ok && !picked.out.trim().is_empty() {
        PathBuf::from(picked.out.trim())
    } else if picked.code == -1 {
        let _ = std::fs::create_dir_all(&docs);
        start
    } else {
        return Err("Not saved.".into());
    };
    std::fs::write(&path, body).map_err(|e| format!("It couldn't be saved: {e}"))?;
    Ok(format!("Saved {}", path.display()))
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Describe(dm) => {
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(|id| block_action(a, id)) {
                return update(a, m);
            }
            return a.describe.update(dm).map(Msg::Describe);
        }
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Tick => {
            if let Some(pl) = a.play.as_mut() {
                let el = pl.started.elapsed().as_secs_f64() * a.prefs.f("speed", 1.0);
                a.pos = pl.from + el;
                let done = pl.child.as_mut().map(|c| matches!(c.try_wait(), Ok(Some(_)))).unwrap_or(true);
                if done {
                    pl.child = None;
                }
            }
        }
        Msg::Poll => {
            a.install = whisper::install_state();
            a.cli = whisper::cli();
            a.models_size = hearth_oss::dir_size(&whisper::models_dir());
        }
        Msg::Loaded(v) => {
            let busy: Vec<(String, String)> = a.recs.iter().filter(|r| !r.busy.is_empty()).map(|r| (r.id.clone(), r.busy.clone())).collect();
            a.recs = v;
            for (id, b) in busy {
                if let Some(r) = a.recs.iter_mut().find(|r| r.id == id) {
                    r.busy = b;
                }
            }
            a.loaded = true;
            if a.open.is_empty() || !a.recs.iter().any(|r| r.id == a.open) {
                a.open = a.recs.first().map(|r| r.id.clone()).unwrap_or_default();
            }
        }
        Msg::Search(q) => a.q = q,
        Msg::Pick(id) => {
            stop_play(a);
            a.play = None;
            a.open = id;
            a.pos = 0.0;
        }
        Msg::Record => {
            if a.recording.is_some() {
                return stop_record(a);
            }
            stop_play(a);
            start_record(a);
            return load();
        }
        Msg::OpenFile => {
            return Task::perform(bg(|| run::blocking(&run::args(&["kdialog", "--getopenfilename", &hearth_ui::paths::home().to_string_lossy(), "audio/wav audio/mpeg audio/ogg audio/flac audio/x-m4a video/mp4 video/webm video/x-matroska"]))), |o| {
                Msg::Picked(o.filter(|o| o.ok).map(|o| o.out.trim().to_string()).unwrap_or_default())
            });
        }
        Msg::Picked(f) => {
            if f.is_empty() {
                return Task::none();
            }
            let name = std::path::Path::new(&f).file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
            let title = name.rsplit_once('.').map(|x| x.0.to_string()).unwrap_or(name.clone());
            let dir = match recs::create(&title, &name) {
                Ok(d) => d,
                Err(e) => {
                    a.said = e;
                    return Task::none();
                }
            };
            let id = dir.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
            a.open = id.clone();
            let Some(cli) = a.cli.clone() else {
                a.said = "Install whisper.cpp first (How it listens).".into();
                let d2 = dir.clone();
                return Task::perform(bg(move || to_wav(&f, &d2.join("audio.wav"))), |_| Msg::Again);
            };
            let how = a.how();
            a.said = format!("Making {name} into words…");
            let id2 = id.clone();
            return Task::batch([
                load(),
                Task::perform(bg(move || to_wav(&f, &dir.join("audio.wav")).and_then(|_| make_words(dir, how, cli))), move |r| Msg::Made(id2.clone(), r.unwrap_or_else(|| Err("it stopped".into())))),
            ]);
        }
        Msg::Made(id, r) => {
            match r {
                Ok(s) => a.said = s,
                Err(e) => {
                    a.said = e.clone();
                    if let Some(rc) = a.recs.iter().find(|x| x.id == id) {
                        let _ = recs::set_meta(&rc.dir, "error", json!(e));
                    }
                }
            }
            return load();
        }
        Msg::Again => return load(),
        Msg::Gpu(g) => a.gpu = g,
        Msg::MakeWords(id) => {
            if let Some(dir) = a.recs.iter().find(|r| r.id == id).map(|r| r.dir.clone()) {
                let _ = recs::set_meta(&dir, "error", json!(""));
                return words_task(a, id, dir);
            }
        }
        Msg::Copy => {
            if let Some(text) = a.rec().map(|r| whisper::to_txt(&r.segs)) {
                a.said = "Copied.".into();
                return iced::clipboard::write(text);
            }
        }
        Msg::Export(ext) => {
            a.menu = None;
            if let Some(r) = a.rec().cloned() {
                return Task::perform(bg(move || export_file(&r, ext)), |r| Msg::Exported(r.unwrap_or_else(|| Err("it stopped".into()))));
            }
        }
        Msg::Exported(r) | Msg::Sent(r) => {
            a.said = match r {
                Ok(s) | Err(s) => s,
            }
        }
        Msg::SendVs => {
            if let Some(r) = a.rec().cloned().filter(|r| r.has_words()) {
                a.said = "Sending to Voice Studio…".into();
                return Task::perform(bg(move || send_vs(r)), |r| Msg::Sent(r.unwrap_or_else(|| Err("it stopped".into()))));
            }
        }
        Msg::Play => {
            if a.playing() {
                stop_play(a);
                a.play = None;
            } else {
                start_play(a);
            }
        }
        Msg::Seek(t) => {
            a.pos = t.max(0.0);
            if a.playing() {
                start_play(a);
            }
        }
        Msg::SeekFrac(f) => {
            let secs = a.rec().map(|r| r.secs).unwrap_or(0.0);
            return update(a, Msg::Seek(secs * f as f64));
        }
        Msg::Speed(s) => {
            a.set("speed", json!(s));
            if a.playing() {
                start_play(a);
            }
        }
        Msg::Menu(m) => a.menu = if a.menu == Some(m) { None } else { Some(m) },
        Msg::Model(id) => {
            a.menu = None;
            a.set("model", json!(id));
        }
        Msg::RunsOn(g) => {
            a.menu = None;
            a.set("gpu", json!(g));
        }
        Msg::Lang(l) => {
            a.menu = None;
            a.set("language", json!(l));
        }
        Msg::Toggle(k) => {
            let d = !matches!(k, "translate");
            let v = !a.prefs.b(k, d);
            a.set(k, json!(v));
        }
        Msg::EditWords => a.words_edit = Some(a.words().join(", ")),
        Msg::Words(s) => a.words_edit = Some(s),
        Msg::WordsDone => {
            if let Some(s) = a.words_edit.take() {
                let v: Vec<String> = s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
                a.set("words", json!(v));
            }
        }
        Msg::Captions => {
            let on = !a.prefs.b("captions", false);
            a.set("captions", json!(on));
            if on {
                captions::start();
            }
        }
        Msg::CapWhere(x) => a.set("captionsWhere", json!(x)),
        Msg::CapSize(s) => a.cap_size = s.round(),
        Msg::CapSizeDone => {
            let s = a.cap_size;
            a.set("captionSize", json!(s));
        }
        Msg::CapListen(x) => a.set("captionsFrom", json!(x)),
        Msg::Install => {
            a.said = "Installing whisper.cpp: it's built for this PC in the background (a few minutes).".into();
            return Task::perform(run::spawn(run::args(&["systemd-run", "--user", &format!("--unit=hearth-listen-install-{}", hearth_oss::now()), "--collect", "--property=Nice=10", "--property=IOSchedulingClass=idle", "/bin/bash", &run::shell_helper("hearth-listen-install")])), |o| Msg::Got("install".into(), o));
        }
        Msg::GetModel(id) => {
            let Some((_, name, size, sha)) = whisper::MODELS.iter().find(|m| m.0 == id).copied().or_else(|| (id == whisper::TDRZ.0).then_some((whisper::TDRZ.0, "Voices (tinydiarize)", "465 MB", whisper::TDRZ.1))) else { return Task::none() };
            a.getting = format!("Getting {name} ({size})…");
            let dir = whisper::models_dir();
            let f = whisper::model_file(id.as_str());
            let url = whisper::model_url(&id);
            let id2 = id.clone();
            return Task::perform(
                bg(move || {
                    let _ = std::fs::create_dir_all(&dir);
                    let part = f.with_extension("bin.part");
                    let o = run::blocking(&run::args(&["curl", "-fL", "--retry", "3", "-C", "-", "-o", &part.to_string_lossy(), &url]));
                    if !o.ok {
                        return o;
                    }
                    let s = run::blocking(&run::args(&["sha1sum", &part.to_string_lossy()]));
                    if !s.out.starts_with(sha) {
                        let _ = std::fs::remove_file(&part);
                        return run::Output { ok: false, code: 1, out: String::new(), err: "the file that came isn't whisper.cpp's (its SHA-1 doesn't match), so it was thrown away".into() };
                    }
                    let _ = std::fs::rename(&part, &f);
                    s
                }),
                move |o| Msg::Got(format!("model:{id2}"), o.unwrap_or(run::Output { ok: false, code: -1, out: String::new(), err: "it stopped".into() })),
            );
        }
        Msg::Got(k, o) => {
            if k == "install" {
                if !o.ok {
                    a.said = format!("The install couldn't start: {}", o.why());
                }
            } else if let Some(id) = k.strip_prefix("model:") {
                a.getting.clear();
                a.said = if o.ok { format!("{} is ready.", whisper::model_name(id)) } else { format!("The model couldn't be got: {}", o.why()) };
                a.models_size = hearth_oss::dir_size(&whisper::models_dir());
            }
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::Close => {
            stop_play(a);
            if a.recording.is_some() {
                let _ = stop_record(a);
            }
            return win::close();
        }
        Msg::Resize(d) => return win::resize(d),
    }
    Task::none()
}

// ---------------------------------------------------------------- the window

fn voice_colour(p: &Pal, v: u8) -> Color {
    match v {
        1 => p.accent,
        2 => w::blue(),
        3 => w::green(),
        4 => w::pink(),
        _ => p.muted,
    }
}

/// "Today 1:12 AM · 4:12", "Yesterday · 2:41", "Nov 24 · 6:08"
fn when_line(ts: i64, secs: f64) -> String {
    let d = hearth_ui::time::local(ts);
    let n = hearth_ui::time::local(hearth_oss::now());
    let y = hearth_ui::time::local(hearth_oss::now() - 86_400);
    let same = |a: hearth_ui::time::Tm, b: hearth_ui::time::Tm| a.year == b.year && a.mon == b.mon && a.day == b.day;
    let day = if same(d, n) {
        format!("Today {}", hearth_ui::time::clock(ts))
    } else if same(d, y) {
        "Yesterday".to_string()
    } else {
        hearth_ui::time::day(ts)
    };
    format!("{day} · {}", whisper::mmss(secs))
}

fn side(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let search = w::search(&p, "Search every word", &a.q, Msg::Search);
    let hits = recs::search(&a.recs, &a.q);
    let mut list = Column::new().spacing(4.0).push(w::cap(&p, "Recordings"));
    let mut shown = 0;
    for r in &a.recs {
        if !a.q.trim().is_empty() && !hits.iter().any(|h| h.0 == r.id) {
            continue;
        }
        shown += 1;
        let on = a.rec().map(|x| x.id == r.id).unwrap_or(false);
        let sub = if !r.busy.is_empty() { r.busy.clone() } else if a.recording.as_ref().map(|x| x.dir == r.dir).unwrap_or(false) { "recording now…".into() } else { when_line(r.when, r.secs) };
        let n_hits = hits.iter().filter(|h| h.0 == r.id && h.1 != usize::MAX).count();
        let mut info = column![t(r.title.clone(), 13.0, p.text), t(sub, 11.0, p.muted)].spacing(3.0);
        if n_hits > 0 {
            info = info.push(t(format!("{n_hits} {} with \"{}\"", if n_hits == 1 { "line" } else { "lines" }, a.q.trim()), 11.0, p.accent));
        }
        let inner = container(row![w::dot(if on { p.accent } else { p.muted }, 16.0), info].spacing(10.0)).padding(10.0).width(Length::Fill);
        list = list.push(mouse_area(w::lit_row(&p, inner.into(), on, 10.0, 56.0)).on_press(Msg::Pick(r.id.clone())).interaction(mouse::Interaction::Pointer));
    }
    if a.loaded && shown == 0 {
        list = list.push(t(if a.q.trim().is_empty() { "Nothing recorded yet. Press Record, or Open a file." } else { "No recording has those words." }, 12.0, p.muted));
    }
    // Edit layout, always to hand (every Hearth app has it)
    list = list.push(ui::gap(1.0, 10.0)).push(
        mouse_area(container(row![ui::glyph(G::Pencil, p.muted, 13.0, 1.8), t("Edit layout", 12.0, p.muted)].spacing(8.0).align_y(iced::Center)).padding(Padding { top: 6.0, right: 4.0, bottom: 6.0, left: 4.0 }))
            .on_press(Msg::Describe(hearth_ui::arrange::edit_layout()))
            .interaction(mouse::Interaction::Pointer),
    );
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "side",
        vec![("search", search), ("list", scrollable(list).height(Length::Fill).into())],
        |ps| column![ps.take("search"), ui::gap(1.0, 12.0), ps.take("list")].into(),
        Msg::Describe,
    );
    container(body).width(Length::Fixed(300.0)).height(Length::Fill).padding(Padding { top: 14.0, right: 12.0, bottom: 14.0, left: 12.0 }).style(w::side_bg(&p)).into()
}

/// Where the words being said right now are in a line (byte range), from their times.
fn now_words(s: &whisper::Seg, pos: f64) -> Option<(usize, usize)> {
    if pos < s.t0 || pos >= s.t1 {
        return None;
    }
    let words: Vec<(usize, usize)> = {
        let mut v = Vec::new();
        let mut start = None;
        for (i, c) in s.text.char_indices() {
            if c.is_whitespace() {
                if let Some(st) = start.take() {
                    v.push((st, i));
                }
            } else if start.is_none() {
                start = Some(i);
            }
        }
        if let Some(st) = start {
            v.push((st, s.text.len()));
        }
        v
    };
    if words.is_empty() {
        return None;
    }
    let n = words.len();
    let times: Vec<f64> = if s.wt.len() == n { s.wt.clone() } else { (0..n).map(|i| s.t0 + (s.t1 - s.t0) * i as f64 / n as f64).collect() };
    let lo = pos - 0.35;
    let hi = pos + 1.6;
    let idx: Vec<usize> = (0..n).filter(|i| times[*i] >= lo && times[*i] <= hi).collect();
    let (f, l) = (*idx.first()?, *idx.last()?);
    let mut end = words[l].1;
    // the mark after a phrase stays outside it, as the board shows it ("same way: Files")
    while end > words[l].0 && s.text[..end].ends_with([':', ',', '.', ';']) {
        end -= 1;
    }
    Some((words[f].0, end))
}

/// The waveform, pressed anywhere to play from there.
struct Wave(w::Bars);
impl canvas::Program<Msg> for Wave {
    type State = ();
    fn update(&self, _s: &mut (), event: &canvas::Event, b: iced::Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let at = cursor.position_in(b)?;
                Some(canvas::Action::publish(Msg::SeekFrac((at.x / b.width.max(1.0)).clamp(0.0, 1.0))).and_capture())
            }
            _ => None,
        }
    }
    fn draw(&self, _s: &(), r: &iced::Renderer, th: &iced::Theme, b: iced::Rectangle, c: mouse::Cursor) -> Vec<canvas::Geometry> {
        <w::Bars as canvas::Program<Msg>>::draw(&self.0, &(), r, th, b, c)
    }
    fn mouse_interaction(&self, _s: &(), b: iced::Rectangle, c: mouse::Cursor) -> mouse::Interaction {
        if c.is_over(b) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

fn line_text<'a>(p: &Pal, text: &str, hl: Option<(usize, usize)>) -> Element<'a, Msg> {
    use iced::widget::text::Span;
    let Some((x, y)) = hl.filter(|(x, y)| x < y && *y <= text.len() && text.is_char_boundary(*x) && text.is_char_boundary(*y)) else {
        return t(text.to_string(), 15.0, p.text).into();
    };
    let base = |s: &str| -> Span<'a, (), iced::Font> { iced::widget::span(s.to_string()).color(p.text).size(15.0) };
    let mid: Span<'a, (), iced::Font> = iced::widget::span(text[x..y].to_string())
        .color(p.text)
        .size(15.0)
        .background(Pal::a(p.accent, 0.2))
        .border(iced::Border { radius: 4.0f32.into(), ..iced::Border::default() })
        .padding(Padding { top: 0.0, right: 3.0, bottom: 0.0, left: 3.0 });
    let spans: Vec<Span<'a, (), iced::Font>> = vec![base(&text[..x]), mid, base(&text[y..])];
    iced::widget::rich_text(spans).into()
}

fn recording_page(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let Some(r) = a.rec() else {
        let msg = if !a.loaded { "Reading your recordings…" } else { "Press Record to make one from your microphone, or Open a file to make any sound or video into words." };
        return container(t(msg, 14.0, p.muted)).padding(30.0).into();
    };
    // its name and buttons
    let mut facts = vec![format!("Recorded in {}", r.source), whisper::mmss(r.secs)];
    if !r.lang.is_empty() {
        facts.push(whisper::lang_name(&r.lang));
    }
    if r.voices() > 1 {
        facts.push(format!("{} voices", r.voices()));
    }
    if r.has_words() && r.took > 0.0 {
        facts.push(format!("made into words in {} s", r.took.round()));
    }
    let has = r.has_words();
    let mut export_c = Column::new().push(w::chip(&p, "Export ▾ .txt .srt .vtt", false, Sz::Md, has.then_some(Msg::Menu("export"))));
    if a.menu == Some("export") {
        export_c = export_c.push(w::menu(&p, vec![("Text (.txt)".into(), false, Msg::Export("txt")), ("Subtitles (.srt)".into(), false, Msg::Export("srt")), ("Web subtitles (.vtt)".into(), false, Msg::Export("vtt"))], 200.0));
    }
    let head = row![
        column![w::h1(&p, r.title.clone()), t(facts.join(" · "), 12.0, p.muted)].spacing(4.0),
        Space::new().width(Length::Fill),
        w::chip(&p, "Copy", false, Sz::Md, has.then_some(Msg::Copy)),
        export_c,
        w::chip(&p, "Send to Voice Studio", true, Sz::Md, has.then_some(Msg::SendVs)),
    ]
    .spacing(10.0)
    .align_y(iced::alignment::Vertical::Top);

    // the player
    let frac = if r.secs > 0.0 { (a.pos / r.secs) as f32 } else { 0.0 };
    let wave = canvas(Wave(w::Bars { vals: if r.peaks.is_empty() { vec![0.05; 160] } else { r.peaks.clone() }, color: w::blue(), head: Some(frac), head_color: p.accent, bright_over: 0.8, centred: false })).width(Length::Fill).height(Length::Fixed(46.0));
    let speed = a.speed();
    let sp = |v: f32, l: &str| (l.to_string(), (speed - v).abs() < 0.01, Some(Msg::Speed(v)));
    let player = w::card(
        &p,
        row![
            w::chip(&p, if a.playing() { "❚❚" } else { "▶" }, false, Sz::Md, Some(Msg::Play)),
            wave,
            t(format!("{} / {}", whisper::mmss(a.pos), whisper::mmss(r.secs)), 12.0, p.muted).wrapping(iced::widget::text::Wrapping::None),
            w::seg(&p, vec![sp(0.75, "0.75×"), sp(1.0, "1×"), sp(1.5, "1.5×")]),
        ]
        .spacing(14.0)
        .align_y(iced::Center),
    )
    .padding(Padding { top: 12.0, right: 16.0, bottom: 12.0, left: 16.0 });

    // what was said
    let mut lines = Column::new();
    if !r.busy.is_empty() {
        lines = lines.push(container(t(r.busy.clone(), 14.0, p.muted)).padding(14.0));
    } else if !has {
        let why = if !r.error.is_empty() { r.error.clone() } else if a.cli.is_none() { "Install whisper.cpp (How it listens) to make this into words.".into() } else { "Not made into words yet.".into() };
        lines = lines.push(container(column![t(why, 14.0, p.muted), w::chip(&p, "Make it into words", true, Sz::Sm, a.cli.as_ref().map(|_| Msg::MakeWords(r.id.clone())))].spacing(10.0)).padding(14.0));
    }
    let q = a.q.trim().to_lowercase();
    for s in &r.segs {
        let now = a.pos >= s.t0 && a.pos < s.t1;
        let hl = if now {
            now_words(s, a.pos)
        } else if !q.is_empty() {
            s.text.to_lowercase().find(&q).map(|i| (i, i + q.len()))
        } else {
            None
        };
        let who: Element<'_, Msg> = if s.voice > 0 { tb(format!("VOICE {}", s.voice), 12.0, voice_colour(&p, s.voice)).into() } else { Space::new().into() };
        let ln = row![
            container(t(whisper::mmss(s.t0), 12.0, p.muted)).width(Length::Fixed(70.0)).padding(Padding { top: 3.0, right: 0.0, bottom: 0.0, left: 0.0 }),
            container(who).width(Length::Fixed(110.0)).padding(Padding { top: 3.0, right: 0.0, bottom: 0.0, left: 0.0 }),
            container(line_text(&p, &s.text, hl)).width(Length::Fill),
        ]
        .spacing(12.0);
        let bg = if now { Pal::mix(p.panel, p.accent, 0.13) } else { Color::TRANSPARENT };
        let cell = container(ln).padding(Padding { top: 10.0, right: 0.0, bottom: 10.0, left: 0.0 }).width(Length::Fill).style(ui::boxed(bg, if now { p.r(8.0) } else { 0.0 }, None));
        lines = lines.push(mouse_area(cell).on_press(Msg::Seek(s.t0)).interaction(mouse::Interaction::Pointer)).push(ui::hline(Pal::mix(p.line, p.bg, 0.4)));
    }
    let mut note = Column::new();
    if !a.said.is_empty() {
        note = note.push(container(t(a.said.clone(), 12.0, p.muted)).padding(Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 }));
    }
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "recording",
        vec![("head", head.into()), ("player", player.into()), ("words", scrollable(column![lines, note].padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 0.0 })).height(Length::Fill).into())],
        |ps| column![ps.take("head"), ui::gap(1.0, 16.0), ps.take("player"), ui::gap(1.0, 8.0), ps.take("words")].into(),
        Msg::Describe,
    )
}

fn how_panel(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let model = a.model();
    let fw = 170.0;
    // model, where it runs, language
    let mut engine = Column::new().spacing(2.0).push(w::h2(&p, "How it listens")).push(ui::gap(1.0, 10.0));
    if a.cli.is_none() {
        let st = &a.install;
        let line = if st.get("running").and_then(Value::as_bool).unwrap_or(false) {
            format!("Installing whisper.cpp {}: {}", whisper::TAG, st.get("step").and_then(Value::as_str).unwrap_or("…"))
        } else if let Some(e) = st.get("error").and_then(Value::as_str).filter(|e| !e.is_empty()) {
            format!("The install stopped: {e}")
        } else {
            format!("whisper.cpp isn't on this PC yet. Hearth builds its {} release for this PC, in your own folder (a few minutes).", whisper::TAG)
        };
        engine = engine.push(w::card2(&p, column![t(line, 12.0, p.text), w::chip(&p, "Install whisper.cpp", true, Sz::Sm, Some(Msg::Install))].spacing(8.0))).push(ui::gap(1.0, 8.0));
    }
    let mut mrow = Column::new().push(w::kv(&p, "Model", w::field(&p, format!("{} ▾", whisper::model_name(&model)), fw, Some(Msg::Menu("model")))));
    if a.menu == Some("model") {
        let items = whisper::MODELS.iter().map(|m| (format!("{} · {}{}", m.1, m.2, if whisper::have_model(m.0) { "" } else { " · get it" }), m.0 == model, Msg::Model(m.0.to_string()))).collect();
        mrow = mrow.push(row![Space::new().width(Length::Fill), w::menu(&p, items, 250.0)]);
    }
    if !whisper::have_model(&model) {
        let busy = !a.getting.is_empty();
        mrow = mrow.push(row![Space::new().width(Length::Fill), w::chip(&p, if busy { a.getting.clone() } else { format!("Get {} ({})", whisper::model_name(&model), whisper::MODELS.iter().find(|m| m.0 == model).map(|m| m.2).unwrap_or("")) }, false, Sz::Sm, (!busy).then_some(Msg::GetModel(model.clone())))]);
    }
    engine = engine.push(mrow);
    let gpu_on = a.prefs.b("gpu", true) && !a.gpu.is_empty();
    let runs = if gpu_on { format!("Nvidia PC · {} ▾", a.gpu) } else { "This PC · processor ▾".to_string() };
    let mut rrow = Column::new().push(w::kv(&p, "Runs on", w::field(&p, runs, fw, Some(Msg::Menu("runs")))));
    if a.menu == Some("runs") {
        let mut items = Vec::new();
        if !a.gpu.is_empty() {
            items.push((format!("Nvidia PC · {} (its graphics card)", a.gpu), gpu_on, Msg::RunsOn(true)));
        }
        items.push(("This PC · processor only".to_string(), !gpu_on, Msg::RunsOn(false)));
        rrow = rrow.push(row![Space::new().width(Length::Fill), w::menu(&p, items, 250.0)]);
    }
    engine = engine.push(rrow);
    let lang = a.prefs.s("language", "auto");
    let mut lrow = Column::new().push(w::kv(&p, "Language", w::field(&p, format!("{} ▾", whisper::lang_name(&lang)), fw, Some(Msg::Menu("lang")))));
    if a.menu == Some("lang") {
        let items = whisper::LANGS.iter().map(|l| (l.1.to_string(), l.0 == lang, Msg::Lang(l.0.to_string()))).collect();
        lrow = lrow.push(row![Space::new().width(Length::Fill), w::menu(&p, items, 200.0)]);
    }
    engine = engine.push(lrow);

    // how it writes
    let voices = a.prefs.b("voices", true);
    let mut writes = Column::new().spacing(2.0);
    writes = writes.push(w::kv(&p, "Tell voices apart", w::sw(&p, voices, Some(Msg::Toggle("voices")))));
    if voices && !whisper::have_model(whisper::TDRZ.0) {
        writes = writes.push(row![Space::new().width(Length::Fill), w::chip(&p, if a.getting.is_empty() { "Get whisper.cpp's voices model (465 MB)".to_string() } else { a.getting.clone() }, false, Sz::Sm, a.getting.is_empty().then_some(Msg::GetModel(whisper::TDRZ.0.to_string())))]);
    }
    writes = writes.push(w::kv(&p, "Commas and capitals", w::sw(&p, a.prefs.b("punctuation", true), Some(Msg::Toggle("punctuation")))));
    writes = writes.push(w::kv(&p, "Translate into English", w::sw(&p, a.prefs.b("translate", false), Some(Msg::Toggle("translate")))));
    let words = a.words();
    let wl = if words.len() > 3 { format!("{} +{}", words[..3].join(", "), words.len() - 3) } else { words.join(", ") };
    match &a.words_edit {
        Some(s) => {
            writes = writes.push(w::kv(&p, "Words it should know", Space::new().into())).push(w::input(&p, "Hearth, Mantel, Ithil…", s, 12.0, Msg::Words).on_submit(Msg::WordsDone)).push(row![Space::new().width(Length::Fill), w::chip(&p, "Done", true, Sz::Sm, Some(Msg::WordsDone))]);
        }
        None => writes = writes.push(w::kv(&p, "Words it should know", w::chip(&p, if wl.is_empty() { "None ✎".to_string() } else { wl }, false, Sz::Sm, Some(Msg::EditWords)))),
    }

    // live captions
    let cap_on = a.prefs.b("captions", false);
    let wh = a.prefs.s("captionsWhere", "bottom");
    let from = a.prefs.s("captionsFrom", "playing");
    let s3 = |v: &'static str, l: &str, cur: &str, f: fn(&'static str) -> Msg| (l.to_string(), cur == v, Some(f(v)));
    let live = column![
        w::cap(&p, "Live captions"),
        w::kv(&p, "Captions for anything playing", w::sw(&p, cap_on, Some(Msg::Captions))),
        w::kv(&p, "Where", w::seg(&p, vec![s3("top", "Top", &wh, Msg::CapWhere), s3("bottom", "Bottom", &wh, Msg::CapWhere), s3("window", "Window", &wh, Msg::CapWhere)])),
        w::kv(
            &p,
            "Size",
            row![slider(12.0..=40.0, a.cap_size, Msg::CapSize).step(1.0f32).on_release(Msg::CapSizeDone).width(Length::Fixed(120.0)).style(w::slider_style(&p, 1.0)), container(t(format!("{} px", a.cap_size as i32), 12.0, p.muted)).width(Length::Fixed(44.0)).align_x(iced::alignment::Horizontal::Right)]
                .spacing(10.0)
                .align_y(iced::Center)
                .into()
        ),
        w::kv(&p, "Listen to", w::seg(&p, vec![s3("mic", "Mic", &from, Msg::CapListen), s3("playing", "What's playing", &from, Msg::CapListen), s3("both", "Both", &from, Msg::CapListen)])),
    ]
    .spacing(2.0);

    let ties = container(
        column![
            row![w::mark("VS", "or", 20.0), tb("Ties to Voice Studio", 12.0, p.text)].spacing(10.0).align_y(iced::Center),
            t("Words come back with their times, so Voice Studio can cut takes by what was said and make subtitles for both video sizes.", 12.0, p.muted),
        ]
        .spacing(6.0),
    )
    .padding(Padding { top: 12.0, right: 14.0, bottom: 12.0, left: 14.0 })
    .width(Length::Fill)
    .style(ui::boxed(Pal::mix(p.panel, p.accent, 0.07), p.r(12.0), Some((Pal::mix(p.line, p.accent, 0.35), 1.0))));
    let where_ = t(format!("Nothing is sent anywhere. Models: {} in ~/.local/share/hearth/listen", hearth_oss::size_words(a.models_size)), 12.0, p.muted);

    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "how",
        vec![("engine", engine.into()), ("writes", writes.into()), ("live", live.into()), ("ties", ties.into()), ("where", where_.into())],
        |ps| column![ps.take("engine"), ps.take("writes"), ps.take("live"), ui::gap(1.0, 16.0), ps.take("ties"), ui::gap(1.0, 14.0), ps.take("where")].into(),
        Msg::Describe,
    );
    container(scrollable(body).height(Length::Fill))
        .width(Length::Fixed(380.0))
        .height(Length::Fill)
        .padding(Padding { top: 18.0, right: 20.0, bottom: 18.0, left: 20.0 })
        .style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.12), 0.0, None))
        .into()
}

/// Hearth Listen's blocks (hearth_ui::blocks): every part of the board is one, moved, sized, hidden or restyled.
use hearth_ui::blocks::{bl, bls, PageDef};
const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Hearth Listen", blocks: &[bl("side", "Recordings", 0, 0, 2, 11), bl("page", "The recording", 2, 0, 7, 11), bl("how", "How it listens", 9, 0, 3, 11), bl("credit", "Built on whisper.cpp (MIT)", 0, 11, 12, 1)] },
    PageDef { id: "side", title: "Recordings", blocks: &[bl("search", "Search every word", 0, 0, 12, 1), bls("list", "The recordings", 0, 1, 12, 11)] },
    PageDef { id: "recording", title: "The recording", blocks: &[bl("head", "Its name and buttons", 0, 0, 12, 1), bl("player", "The player", 0, 1, 12, 1), bls("words", "What was said", 0, 2, 12, 10)] },
    PageDef {
        id: "how",
        title: "How it listens",
        blocks: &[bl("engine", "Model, where it runs, language", 0, 0, 12, 3), bl("writes", "How it writes", 0, 3, 12, 3), bl("live", "Live captions", 0, 6, 12, 3), bl("ties", "Ties to Voice Studio", 0, 9, 12, 2), bl("where", "Nothing is sent anywhere", 0, 11, 12, 1)],
    },
];

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    studio_values(a);
    let mut items: Vec<(&'static str, Element<'_, Msg>)> = vec![
        ("side", hearth_ui::studio::made(&a.describe, "window", "side", || side(a))),
        ("page", container(hearth_ui::studio::made(&a.describe, "window", "page", || recording_page(a))).padding(Padding { top: 20.0, right: 28.0, bottom: 12.0, left: 28.0 }).width(Length::Fill).height(Length::Fill).into()),
        ("how", hearth_ui::studio::made(&a.describe, "window", "how", || how_panel(a))),
        ("credit", w::credit(&p, "Hearth Listen", "Built on whisper.cpp (MIT)")),
    ];
    for (id, which) in [("pane:recordings", "recordings"), ("pane:captions", "captions"), ("pane:words", "words")] {
        if a.describe.wants_pane(which) {
            items.push((id, studio_pane(a, which)));
        }
    }
    let base = hearth_ui::arrange::parts(&a.describe, &p, "window", items, |ps| column![row![ps.take("side"), ps.take("page"), ps.take("how")].height(Length::Fill), ps.take("credit")].into(), Msg::Describe);
    // the title bar: its name, what it is, Record and Open a file
    let rec_label = match &a.recording {
        Some(r) => format!("■ Stop · {}", whisper::mmss(r.started.elapsed().as_secs_f64())),
        None => "● Record".to_string(),
    };
    let middle: Element<'_, Msg> = container(
        row![t("· speech to words, on your own PC", 14.0, p.muted), Space::new().width(Length::Fill), w::chip(&p, rec_label, true, Sz::Sm, Some(Msg::Record)), w::chip(&p, "Open a file", false, Sz::Sm, Some(Msg::OpenFile))].spacing(8.0).align_y(iced::Center),
    )
    .width(Length::FillPortion(30))
    .into();
    let bar = hearth_ui::describe::title_bar(&p, &a.describe, G::Sound, "Hearth Listen", Some(middle), TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close }, Pal::mix(p.bg, Color::BLACK, 0.3), Msg::Describe(hearth_ui::describe::DMsg::Open));
    // the board's LI letter mark over the title bar's own mark (only while its title bar is as it was made)
    let bar: Element<'_, Msg> = if a.describe.zone("titlebar").is_none() { iced::widget::stack![bar, container(w::mark("LI", "bl", 22.0)).padding(Padding { top: 9.0, right: 0.0, bottom: 0.0, left: 11.0 })].into() } else { bar };
    let all = container(column![bar, ui::hline(p.line), base]).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, p.window_corner, Some((p.line, 1.0))));
    ui::edges(all.into(), Msg::Resize)
}

// ------------------------------------------------------------------ drag and drop (the same in every Hearth app)
use hearth_ui::studio::{tile, Kit};
/// Hearth Listen's blocks window: its buttons, its panes, its info.
const KIT: Kit = Kit {
    name: "Listen",
    buttons: &[
        tile("record", "Record", G::Sound),
        tile("open-file", "Open a file", G::Folder),
        tile("play", "Play / pause", G::Play),
        tile("copy", "Copy the words", G::Doc),
        tile("export-txt", "Export .txt", G::Down),
        tile("export-srt", "Export .srt", G::Down),
        tile("export-vtt", "Export .vtt", G::Down),
        tile("send-vs", "Send to Voice Studio", G::Arrow),
        tile("captions", "Live captions on / off", G::Display),
        tile("slower", "Play at 0.75×", G::Clock),
        tile("normal", "Play at 1×", G::Clock),
        tile("faster", "Play at 1.5×", G::Clock),
        tile("translate", "Translate into English", G::Sync),
    ],
    panes: &[tile("recordings", "Your recordings", G::List), tile("captions", "The newest caption", G::Display), tile("words", "Words it should know", G::Pencil)],
    info: &[tile("recordings", "Recordings", G::List), tile("length", "Its length", G::Clock), tile("voices", "Voices", G::Users), tile("model", "Model", G::Cog), tile("models", "Models on this PC", G::Drive)],
    pane_page: "window",
    bars: &[],
    menus: &[],
};

fn block_action(_a: &App, id: &str) -> Option<Msg> {
    Some(match id {
        "record" => Msg::Record,
        "open-file" => Msg::OpenFile,
        "play" => Msg::Play,
        "copy" => Msg::Copy,
        "export-txt" => Msg::Export("txt"),
        "export-srt" => Msg::Export("srt"),
        "export-vtt" => Msg::Export("vtt"),
        "send-vs" => Msg::SendVs,
        "captions" => Msg::Captions,
        "slower" => Msg::Speed(0.75),
        "normal" => Msg::Speed(1.0),
        "faster" => Msg::Speed(1.5),
        "translate" => Msg::Toggle("translate"),
        _ => return None,
    })
}

fn studio_values(a: &App) {
    let r = a.rec();
    a.describe.values(vec![
        ("recordings", a.recs.len().to_string()),
        ("length", r.map(|r| whisper::mmss(r.secs)).unwrap_or_default()),
        ("voices", r.map(|r| r.voices().to_string()).unwrap_or_default()),
        ("model", whisper::model_name(&a.model())),
        ("models", hearth_oss::size_words(a.models_size)),
    ]);
}

fn studio_pane<'a>(a: &'a App, which: &str) -> Element<'a, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(6.0);
    match which {
        "recordings" => {
            for r in a.recs.iter().take(12) {
                c = c.push(mouse_area(row![t(ui::fit(&r.title, 36), 13.0, p.text), Space::new().width(Length::Fill), t(whisper::mmss(r.secs), 12.0, p.muted)].spacing(10.0)).on_press(Msg::Pick(r.id.clone())).interaction(mouse::Interaction::Pointer));
            }
        }
        "captions" => {
            let last = captions::last_line();
            c = c.push(t(if last.is_empty() { "No captions yet: turn on Captions for anything playing.".to_string() } else { last }, 14.0, p.text));
        }
        _ => {
            for wd in a.words() {
                c = c.push(t(wd, 13.0, p.text));
            }
        }
    }
    scrollable(c).height(Length::Fill).into()
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme)];
    if a.playing() || a.recording.is_some() {
        s.push(iced::time::every(std::time::Duration::from_millis(200)).map(|_| Msg::Tick));
    }
    let installing = a.install.get("running").and_then(Value::as_bool).unwrap_or(false);
    if installing || !a.getting.is_empty() || a.recs.iter().any(|r| !r.busy.is_empty()) {
        s.push(iced::time::every(std::time::Duration::from_secs(2)).map(|_| Msg::Poll));
    }
    Subscription::batch(s)
}

fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::handle_cli(
        "hearth-listen-app",
        "Hearth Listen: speech to words on your own PC (built on whisper.cpp, MIT).\n  --open ID   a recording\n  --at SECS   the player there\n  --captions  the live captions bar",
    ) {
        return Ok(());
    }
    if app::flag("--captions") {
        return captions::run();
    }
    let w = app::window("hearth-listen", (1840.0, 840.0), (1100.0, 600.0));
    iced::application(App::new, update, shown).title(|_: &App| "Hearth Listen".to_string()).subscription(subscription).theme(|a: &App| app::iced_theme(&a.p)).window(w).default_font(app::INTER).run()
}
