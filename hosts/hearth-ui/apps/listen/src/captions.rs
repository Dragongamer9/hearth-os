//! Live captions (`hearth-listen-app --captions`): a small bar on top of everything, at the top or the bottom of the
//! screen (or a window of its own), with what's being said in anything playing, your microphone, or both. The sound
//! comes from PipeWire's PulseAudio side (parec: the default output's monitor, the default microphone), 16 kHz mono;
//! every few seconds, when there's sound, whisper.cpp writes the last moments down. Nothing is kept: the sound lives
//! only in memory and in one small file in your runtime folder that the next moment overwrites.
//!
//! It follows Listen's choices while it runs (Where, Size, Listen to) and ends by itself when captions are turned
//! off, or with ✕ (which turns them off). Quiet sound: whisper.cpp isn't run at all.

use crate::whisper;
use hearth_oss::prefs::Prefs;
use hearth_oss::w::{self, Sz, Tg};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::ui::{self, t};
use hearth_ui::Pal;
use iced::widget::{container, mouse_area, row};
use iced::{window, Color, Element, Length, Padding, Point, Size, Subscription, Task};
use serde_json::{json, Value};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const RATE: usize = 16000;
const KEEP: usize = RATE * 12;

fn rt() -> PathBuf {
    let d = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("hearth-listen");
    let _ = std::fs::create_dir_all(&d);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o700));
    }
    d
}

/// Running already (its pid file names a live process)?
pub fn running() -> bool {
    std::fs::read_to_string(rt().join("captions.pid")).ok().and_then(|s| s.trim().parse::<i32>().ok()).map(|pid| unsafe { libc::kill(pid, 0) } == 0).unwrap_or(false)
}

/// Listen turned captions on: the bar starts (once).
pub fn start() {
    if running() {
        return;
    }
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe).arg("--captions").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    }
}

/// The newest caption (for Listen's "The newest caption" pane).
pub fn last_line() -> String {
    std::fs::read_to_string(rt().join("caption.txt")).unwrap_or_default().trim().to_string()
}

#[derive(Debug, Clone)]
enum M {
    Tick,
    Heard(String),
    From(String),
    Pause,
    Close,
    Drag,
    Theme,
}

struct Cap {
    p: Pal,
    prefs: Prefs,
    bufs: Vec<Arc<Mutex<Vec<i16>>>>,
    kids: Vec<Child>,
    listening: String,
    paused: bool,
    old: String,
    new: String,
    from: String,
    busy: bool,
    ticks: u32,
    problem: String,
    /// CI's pictures: its words from $HEARTH_LISTEN_CAPTION ("old|new|from"), nothing heard
    fixed: bool,
}

impl Cap {
    fn listen(&mut self) {
        for mut k in self.kids.drain(..) {
            let _ = k.kill();
            let _ = k.wait();
        }
        self.bufs.clear();
        let from = self.prefs.s("captionsFrom", "playing");
        self.listening = from.clone();
        let devs: Vec<&str> = match from.as_str() {
            "mic" => vec!["@DEFAULT_SOURCE@"],
            "both" => vec!["@DEFAULT_MONITOR@", "@DEFAULT_SOURCE@"],
            _ => vec!["@DEFAULT_MONITOR@"],
        };
        for d in devs {
            let child = Command::new("parec").args(["--raw", "--format=s16le", "--rate=16000", "--channels=1", "-d", d]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn();
            let Ok(mut child) = child else {
                self.problem = "Live captions need PipeWire's PulseAudio tools (parec).".into();
                return;
            };
            let buf = Arc::new(Mutex::new(Vec::<i16>::with_capacity(KEEP)));
            if let Some(mut out) = child.stdout.take() {
                let b = buf.clone();
                let _ = std::thread::Builder::new().name("hearth-listen-hear".into()).spawn(move || {
                    let mut chunk = [0u8; 6400];
                    while let Ok(n) = out.read(&mut chunk) {
                        if n == 0 {
                            break;
                        }
                        if let Ok(mut v) = b.lock() {
                            v.extend(chunk[..n - n % 2].chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])));
                            let over = v.len().saturating_sub(KEEP);
                            if over > 0 {
                                v.drain(..over);
                            }
                        }
                    }
                });
            }
            self.kids.push(child);
            self.bufs.push(buf);
        }
    }

    /// The last few seconds, the sources mixed.
    fn last(&self, secs: usize) -> Vec<i16> {
        let n = RATE * secs;
        let mut out: Vec<i32> = vec![0; n];
        let mut have = 0;
        for b in &self.bufs {
            if let Ok(v) = b.lock() {
                let take = v.len().min(n);
                have = have.max(take);
                for (i, s) in v[v.len() - take..].iter().enumerate() {
                    out[n - take + i] += *s as i32;
                }
            }
        }
        out[n - have..].iter().map(|x| (*x).clamp(-32768, 32767) as i16).collect()
    }
}

fn fixture() -> Option<(String, String, String)> {
    let s = std::env::var("HEARTH_LISTEN_CAPTION").ok()?;
    let mut it = s.splitn(3, '|');
    Some((it.next()?.to_string(), it.next().unwrap_or("").to_string(), it.next().unwrap_or("").to_string()))
}

fn boot() -> (Cap, Task<M>) {
    let _ = std::fs::write(rt().join("captions.pid"), std::process::id().to_string());
    let mut c = Cap { p: hearth_ui::theme::load(), prefs: Prefs::load("listen"), bufs: Vec::new(), kids: Vec::new(), listening: String::new(), paused: false, old: String::new(), new: String::new(), from: String::new(), busy: false, ticks: 0, problem: String::new(), fixed: false };
    if let Some((o, n, f)) = fixture() {
        c.old = o;
        c.new = n;
        c.from = f;
        c.fixed = true;
        return (c, Task::none());
    }
    if whisper::cli().is_none() {
        c.problem = "Install whisper.cpp in Hearth Listen first.".into();
    } else {
        c.listen();
    }
    (c, Task::done(M::From(String::new())))
}

fn model_for_captions(gpu: bool, chosen: &str) -> Option<String> {
    let mut order: Vec<&str> = Vec::new();
    if gpu {
        order.push(chosen);
    }
    order.extend(["large-v3-turbo-q5_0", "small", "base", "tiny", chosen]);
    order.into_iter().find(|m| whisper::have_model(m)).map(str::to_string)
}

fn hear(wav: PathBuf, model: String, lang: String, gpu: bool) -> String {
    let Some(cli) = whisper::cli() else { return String::new() };
    let mut a = vec![cli.to_string_lossy().to_string(), "-m".into(), whisper::model_file(&model).to_string_lossy().to_string(), "-f".into(), wav.to_string_lossy().to_string(), "-nt".into(), "-np".into(), "-l".into(), lang];
    if !gpu {
        a.push("-ng".into());
    }
    let o = run::blocking(&a);
    o.out.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('[')).collect::<Vec<_>>().join(" ")
}

/// What's playing now, in a word (its program's name), from PipeWire's PulseAudio side.
fn playing_now() -> String {
    let o = run::blocking(&run::args(&["pactl", "-f", "json", "list", "sink-inputs"]));
    let j: Value = serde_json::from_str(&o.out).unwrap_or(Value::Null);
    j.as_array()
        .and_then(|a| a.iter().find(|s| !s.get("corked").and_then(Value::as_bool).unwrap_or(false)))
        .and_then(|s| s.pointer("/properties/application.name").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default()
}

fn update(c: &mut Cap, m: M) -> Task<M> {
    match m {
        M::Theme => c.p = hearth_ui::theme::load(),
        M::Drag => return win::drag(),
        M::Pause => {
            c.paused = !c.paused;
            if c.paused {
                for b in &c.bufs {
                    if let Ok(mut v) = b.lock() {
                        v.clear();
                    }
                }
            }
        }
        M::Close => {
            let mut pr = Prefs::load("listen");
            let _ = pr.set("captions", json!(false));
            return quit(c);
        }
        M::Heard(s) => {
            c.busy = false;
            let s = s.trim().to_string();
            if !s.is_empty() && s != c.new {
                c.old = std::mem::take(&mut c.new);
                c.new = s;
                let _ = std::fs::write(rt().join("caption.txt"), format!("{} {}", c.old, c.new));
            }
        }
        M::From(s) => c.from = s,
        M::Tick => {
            if c.fixed {
                return Task::none();
            }
            c.prefs = Prefs::load("listen");
            if !c.prefs.b("captions", false) {
                return quit(c);
            }
            if c.prefs.s("captionsFrom", "playing") != c.listening && c.problem.is_empty() {
                c.listen();
            }
            c.ticks += 1;
            let mut t = Vec::new();
            if c.ticks % 4 == 1 && c.listening != "mic" {
                t.push(Task::perform(bg(playing_now), |s| M::From(s.unwrap_or_default())));
            }
            if c.paused || c.busy || !c.problem.is_empty() {
                return Task::batch(t);
            }
            let pcm = c.last(6);
            let loud = pcm.iter().rev().take(RATE * 2).map(|x| (*x as f64).powi(2)).sum::<f64>() / (RATE * 2) as f64;
            if pcm.len() < RATE || loud.sqrt() < 180.0 {
                return Task::batch(t);
            }
            let gpu = c.prefs.b("gpu", true);
            let Some(model) = model_for_captions(gpu, &c.prefs.s("model", "large-v3-turbo")) else {
                c.problem = "Get a model in Hearth Listen first.".into();
                return Task::batch(t);
            };
            let f = rt().join("captions.wav");
            if std::fs::write(&f, whisper::wav_bytes(&pcm, RATE as u32, 1)).is_err() {
                return Task::batch(t);
            }
            c.busy = true;
            let lang = c.prefs.s("language", "auto");
            t.push(Task::perform(bg(move || hear(f, model, lang, gpu)), |s| M::Heard(s.unwrap_or_default())));
            return Task::batch(t);
        }
    }
    Task::none()
}

fn quit(c: &mut Cap) -> Task<M> {
    for mut k in c.kids.drain(..) {
        let _ = k.kill();
        let _ = k.wait();
    }
    let _ = std::fs::remove_file(rt().join("captions.pid"));
    let _ = std::fs::remove_file(rt().join("captions.wav"));
    win::close()
}

fn bg<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> impl std::future::Future<Output = Option<T>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-listen-cap".into()).spawn(move || {
        let _ = tx.send(f());
    });
    async move { rx.await.ok() }
}

fn view(c: &Cap) -> Element<'_, M> {
    use iced::widget::text::Span;
    let p = c.p;
    let size = c.prefs.f("captionSize", 28.0).clamp(12.0, 40.0) as f32 * 24.0 / 28.0;
    let words: Element<'_, M> = if !c.problem.is_empty() {
        t(c.problem.clone(), 16.0, p.muted).into()
    } else if c.old.is_empty() && c.new.is_empty() {
        t(if c.paused { "Paused" } else { "Listening…" }, size, p.muted).into()
    } else {
        let spans: Vec<Span<'_, (), iced::Font>> = vec![iced::widget::span(format!("{} ", c.old)).color(p.text).size(size), iced::widget::span(c.new.clone()).color(p.muted).size(size)];
        iced::widget::rich_text(spans).into()
    };
    let from = match c.listening.as_str() {
        "mic" => "from: your microphone".to_string(),
        _ if !c.from.is_empty() => format!("from: {}", c.from),
        _ => String::new(),
    };
    let bar = container(
        row![
            w::tag(&p, "Live captions", Tg::Acc),
            container(words).width(Length::Fill),
            t(from, 12.0, p.muted),
            w::chip(&p, if c.paused { "Go on" } else { "Pause" }, false, Sz::Sm, Some(M::Pause)),
            w::chip(&p, "✕", false, Sz::Sm, Some(M::Close)),
        ]
        .spacing(18.0)
        .align_y(iced::Center),
    )
    .padding(Padding { top: 0.0, right: 24.0, bottom: 0.0, left: 24.0 })
    .width(Length::Fill)
    .height(Length::Fill)
    .style(ui::boxed(Color { a: 0.88, ..Pal::mix(p.bg, Color::BLACK, 0.3) }, 16.0, Some((p.line, 1.0))));
    mouse_area(container(bar).width(Length::Fill).height(Length::Fill)).on_press(M::Drag).into()
}

fn subscription(c: &Cap) -> Subscription<M> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| M::Theme)];
    if !c.fixed {
        s.push(iced::time::every(Duration::from_millis(2500)).map(|_| M::Tick));
    }
    Subscription::batch(s)
}

fn at_bottom(win: Size, mon: Size) -> Point {
    Point::new(((mon.width - win.width) / 2.0).max(0.0), (mon.height - win.height - 66.0).max(0.0))
}
fn at_top(win: Size, mon: Size) -> Point {
    Point::new(((mon.width - win.width) / 2.0).max(0.0), 60.0)
}

pub fn run() -> iced::Result {
    let prefs = Prefs::load("listen");
    // see-through around the rounded bar (hearth-ui's softbuffer with alpha, on tiny-skia), as Capture's indicator
    std::env::set_var("HEARTH_SOFTBUFFER_ALPHA", "1");
    std::env::set_var("ICED_BACKEND", "tiny-skia");
    let mut s = app::window("hearth-listen-captions", (1240.0, 84.0), (600.0, 60.0));
    s.transparent = true;
    match prefs.s("captionsWhere", "bottom").as_str() {
        "window" => s.resizable = true,
        "top" => {
            s.level = window::Level::AlwaysOnTop;
            s.resizable = false;
            s.position = window::Position::SpecificWith(at_top);
        }
        _ => {
            s.level = window::Level::AlwaysOnTop;
            s.resizable = false;
            s.position = window::Position::SpecificWith(at_bottom);
        }
    }
    iced::application(boot, update, view)
        .title(|_: &Cap| "Live captions".to_string())
        .subscription(subscription)
        .theme(|c: &Cap| app::iced_theme(&c.p))
        .style(|c: &Cap, _t: &iced::Theme| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: c.p.text })
        .window(s)
        .default_font(app::INTER)
        .run()
}
