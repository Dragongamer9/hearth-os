//! The picture, sound, screenshots and recording, through GStreamer: one pipeline while a card is open.
//!
//!   v4l2src (MJPEG or YUYV at the mode you chose)
//!     → MJPEG decoded by the graphics card (vajpegdec + vapostproc, VA-API) where it can, else jpegdec (the CPU)
//!     → tee → [the window: RGBA into an appsink, at most one frame waiting: Low delay drops late frames]
//!            → [while recording: H.264 (vah264enc, else openh264enc / x264enc) into MP4]
//!   the card's USB microphone (pipewiresrc) → tee → volume (mute) → pipewiresink (this PC's speakers)
//!                                                  → [while recording: AAC (or Opus) into the same MP4]
//!
//! The delay readout is measured on every frame: from when the card's frame reached this PC (v4l2's own capture time
//! on the pipeline's clock) to when the window takes it, plus one screen refresh (it's on screen at the next one).
//!
//! No signal: no frames for 2 s, or nothing but a blank (one-colour) picture from the start (1.5 s), or for 10 s
//! after a real picture (a game's black loading screen is shorter). Then the window waits with the card's lightest
//! mode (a few small frames a second, or none at all when the card sends nothing) and comes back by itself on the
//! first real picture. Nothing here polls: threads sleep in GStreamer's bus or on a channel.
//!
//! CI's stand-in card (card.rs fake_picture): an appsrc sends its JPEG as the card's MJPEG at the mode's frame rate
//! (a thread reads it again when it changes, so CI can make the signal arrive), and `audiotestsrc` stands in for the microphone; the rest of the pipeline is the same.

use crate::card::Card;
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The pipeline is running: what it asked the card for and how it decodes ("graphics card" / "CPU").
    Started { format: String, w: u32, h: u32, fps: u32, decoder: String, watching: bool },
    /// A new frame is waiting (frame()).
    Frame,
    /// A picture is coming in (true) or not (false).
    Signal(bool),
    /// It stopped by itself (the card was unplugged, or GStreamer failed), in plain words.
    Failed(String),
    Recording(Result<PathBuf, String>),
    /// A recording was finished and closed (its file), or couldn't be.
    Recorded(Result<PathBuf, String>),
}

#[derive(Debug, Clone)]
pub struct Ask {
    pub card: Card,
    pub format: String,
    pub w: u32,
    pub h: u32,
    pub fps: u32,
    pub low_delay: bool,
    /// The card's sound (a PipeWire source), if it has one.
    pub sound: Option<String>,
    pub play_sound: bool,
    pub volume: u32,
    pub muted: bool,
    /// Waiting for a signal: the card's lightest mode, and Signal(true) on the first real picture.
    pub watching: bool,
}

pub struct Frame {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
    /// From the card to here (ns), and when it was put here.
    pub delay_ns: u64,
    pub at: Instant,
}

static EVENTS: OnceLock<(UnboundedSender<(u64, Event)>, Mutex<Option<UnboundedReceiver<(u64, Event)>>>)> = OnceLock::new();
static GEN: AtomicU64 = AtomicU64::new(0);
static FRAME: Mutex<Option<Frame>> = Mutex::new(None);
static PENDING: AtomicBool = AtomicBool::new(false);

fn bus() -> &'static (UnboundedSender<(u64, Event)>, Mutex<Option<UnboundedReceiver<(u64, Event)>>>) {
    EVENTS.get_or_init(|| {
        let (tx, rx) = unbounded();
        (tx, Mutex::new(Some(rx)))
    })
}

fn send(gen: u64, e: Event) {
    let _ = bus().0.unbounded_send((gen, e));
}

/// The window's subscription: the running pipeline's events (nothing when none runs). Each carries its pipeline's
/// number, so an old pipeline's last words are told apart from a new one's.
pub fn events() -> impl futures::Stream<Item = (u64, Event)> {
    let rx = bus().1.lock().unwrap().take();
    match rx {
        Some(r) => futures::stream::StreamExt::boxed(r),
        None => futures::stream::StreamExt::boxed(futures::stream::pending()),
    }
}

/// The number of the pipeline started last.
pub fn current() -> u64 {
    GEN.load(Ordering::Relaxed)
}

/// The newest frame (the window shows it, then waits for the next Frame event).
pub fn frame() -> Option<Frame> {
    PENDING.store(false, Ordering::Release);
    FRAME.lock().unwrap().take()
}

/// Whether the stand-in card is in use (CI).
pub fn stand_in(card: &Card) -> bool {
    card.fake_picture.is_some()
}

/// Is a blank picture (one colour all over: what cards send without HDMI)? A grid of 8 × 8 points.
pub fn blank(rgba: &[u8], w: u32, h: u32) -> bool {
    if w == 0 || h == 0 || rgba.len() < (w * h * 4) as usize {
        return true;
    }
    let (mut lo, mut hi) = ([255u8; 3], [0u8; 3]);
    for gy in 0..8u32 {
        for gx in 0..8u32 {
            let x = (w - 1) * (2 * gx + 1) / 16;
            let y = (h - 1) * (2 * gy + 1) / 16;
            let i = ((y * w + x) * 4) as usize;
            for c in 0..3 {
                lo[c] = lo[c].min(rgba[i + c]);
                hi[c] = hi[c].max(rgba[i + c]);
            }
        }
    }
    (0..3).all(|c| hi[c].saturating_sub(lo[c]) < 10)
}

/// Which decoder: "va" (the graphics card) or "cpu". HEARTH_CAPTURE_DECODER picks one; a VA decoder that failed
/// once isn't tried again until the app opens again.
pub static VA_FAILED: AtomicBool = AtomicBool::new(false);

/// The pipeline's description (also checked by `cargo test`). `va`: decode with the graphics card.
pub fn describe(ask: &Ask, va: bool, have_pw: bool) -> String {
    let fake = ask.card.fake_picture.as_ref();
    let (fmt, w, h, fps) = if ask.watching { ask.card.lightest().unwrap_or(("MJPG".into(), ask.w, ask.h, 5)) } else { (ask.format.clone(), ask.w, ask.h, ask.fps) };
    let mjpg = fmt == "MJPG" || fake.is_some();
    let src = match fake {
        Some(_) => format!("appsrc name=src is-live=true do-timestamp=true format=time caps=\"image/jpeg,framerate={fps}/1\" ! jpegparse"),
        None if mjpg => format!("v4l2src name=src device=\"{}\" ! image/jpeg,width={w},height={h},framerate={fps}/1", ask.card.dev.display()),
        None => format!("v4l2src name=src device=\"{}\" ! video/x-raw,format=YUY2,width={w},height={h},framerate={fps}/1", ask.card.dev.display()),
    };
    let dec = match (mjpg, va) {
        (true, true) => "vajpegdec name=dec",
        (true, false) => "jpegdec name=dec",
        (false, true) => "identity name=dec",
        (false, false) => "identity name=dec",
    };
    // HEARTH_CAPTURE_OUT=WxH: the window's picture at that size (CI's checks of how the graphics card draws it).
    let out = std::env::var("HEARTH_CAPTURE_OUT").ok().and_then(|s| crate::prefs::parse_size(&s)).map(|(w, h)| format!(",width={w},height={h}")).unwrap_or_default();
    let scale = if out.is_empty() { "" } else { "videoscale ! " };
    let to_rgba = if va { format!("vapostproc ! video/x-raw,format=RGBA{out}") } else { format!("videoconvert n-threads=2 ! {scale}video/x-raw,format=RGBA{out}") };
    let view = if ask.low_delay {
        format!("vt. ! queue name=vq max-size-buffers=1 max-size-bytes=0 max-size-time=0 leaky=downstream ! {to_rgba} ! appsink name=pic sync=false max-buffers=1 drop=true")
    } else {
        format!("vt. ! queue name=vq max-size-buffers=4 max-size-bytes=0 max-size-time=0 ! {to_rgba} ! appsink name=pic sync=true max-buffers=2 drop=false")
    };
    let mut d = format!("{src} ! {dec} ! tee name=vt allow-not-linked=true {view}");
    if !ask.watching {
        if let Some(s) = &ask.sound {
            let asrc = if fake.is_some() {
                "audiotestsrc name=asrc is-live=true wave=sine volume=0.05".to_string()
            } else if have_pw {
                format!("pipewiresrc name=asrc target-object=\"{}\" do-timestamp=true stream-properties=\"props,node.latency=(string)256/48000\"", s.replace('"', ""))
            } else {
                format!("pulsesrc name=asrc device=\"{}\"", s.replace('"', ""))
            };
            d.push_str(&format!(" {asrc} ! audioconvert ! audioresample ! audio/x-raw,rate=48000,channels=2 ! queue max-size-time=60000000 max-size-buffers=0 max-size-bytes=0 leaky=downstream ! tee name=at allow-not-linked=true"));
            if ask.play_sound {
                let sink = if fake.is_some() { "fakesink name=asink sync=true" } else if have_pw { "pipewiresink name=asink sync=false stream-properties=\"props,node.latency=(string)256/48000\"" } else { "autoaudiosink name=asink sync=false" };
                d.push_str(&format!(" at. ! queue max-size-time=60000000 max-size-buffers=0 max-size-bytes=0 leaky=downstream ! volume name=vol volume={:.2} mute={} ! {sink}", ask.volume.min(100) as f64 / 100.0, ask.muted));
            }
        }
    }
    d
}

/// The recording's own part of the pipeline (added while it runs): H.264 and the sound into MP4.
pub fn describe_rec(file: &Path, va_decoded: bool, venc: &str, aenc: Option<&str>) -> String {
    let mut conv = String::new();
    if va_decoded {
        conv.push_str("vapostproc ! video/x-raw,format=NV12 ! ");
    }
    let enc = match venc {
        "vah264enc" | "vah264lpenc" => format!("{}videoconvert ! video/x-raw,format=NV12 ! {venc} bitrate=30000 key-int-max=120", conv),
        "openh264enc" => format!("{}videoconvert ! video/x-raw,format=I420 ! openh264enc bitrate=30000000 rate-control=bitrate complexity=low", conv),
        _ => format!("{}videoconvert ! video/x-raw,format=I420 ! x264enc bitrate=30000 speed-preset=veryfast tune=zerolatency key-int-max=120", conv),
    };
    let mut d = format!(
        "queue name=rvq max-size-buffers=0 max-size-bytes=0 max-size-time=3000000000 ! {enc} ! h264parse ! queue ! mp4mux name=mux fragment-duration=1000 ! filesink name=rfile location=\"{}\"",
        file.display().to_string().replace('"', "")
    );
    if let Some(a) = aenc {
        let ae = match a {
            "fdkaacenc" => "fdkaacenc bitrate=192000 ! aacparse",
            "avenc_aac" => "avenc_aac bitrate=192000 ! aacparse",
            "voaacenc" => "voaacenc bitrate=192000 ! aacparse",
            _ => "opusenc bitrate=192000 ! opusparse",
        };
        d.push_str(&format!(" queue name=raq max-size-buffers=0 max-size-bytes=0 max-size-time=3000000000 ! audioconvert ! audioresample ! {ae} ! queue ! mux."));
    }
    d
}

/// What a recording or its thumbnail needs to know about a file (Recordings & screenshots).
pub struct Probe {
    pub secs: u64,
    pub w: u32,
    pub h: u32,
    pub fps: u32,
    pub tw: u32,
    pub th: u32,
    pub rgba: Vec<u8>,
}

#[cfg(not(feature = "media"))]
mod imp {
    use super::*;
    pub fn start(_ask: Ask) {
        let g = GEN.fetch_add(1, Ordering::Relaxed) + 1;
        send(g, Event::Failed("This copy of Capture Card was built without its player (GStreamer).".into()));
    }
    pub fn stop() {}
    pub fn set_volume(_v: u32, _muted: bool) {}
    pub fn running() -> bool {
        false
    }
    pub fn record_start(_f: PathBuf) {}
    pub fn record_stop() {}
    pub fn recording() -> Option<(Instant, PathBuf)> {
        None
    }
    pub fn probe(_p: &Path) -> Option<Probe> {
        None
    }
    pub fn encoders() -> (Option<String>, Option<String>) {
        (None, None)
    }
}

#[cfg(feature = "media")]
mod imp {
    use super::*;
    use gstreamer as gst;
    use gstreamer::prelude::*;
    use std::sync::Arc;
    use std::time::Duration;

    struct Running {
        gen: u64,
        pipeline: gst::Pipeline,
        va: bool,
        rec: Option<Rec>,
    }

    struct Rec {
        bin: gst::Bin,
        pads: Vec<(gst::Element, gst::Pad)>,
        file: PathBuf,
        since: Instant,
    }

    static RUN: Mutex<Option<Running>> = Mutex::new(None);

    fn init() -> bool {
        static OK: OnceLock<bool> = OnceLock::new();
        *OK.get_or_init(|| gst::init().is_ok())
    }

    fn have(f: &str) -> bool {
        gst::ElementFactory::find(f).is_some()
    }

    /// The encoders this PC has for recording: (video, sound).
    pub fn encoders() -> (Option<String>, Option<String>) {
        if !init() {
            return (None, None);
        }
        let v = ["vah264enc", "vah264lpenc", "openh264enc", "x264enc"].into_iter().find(|f| have(f)).map(String::from);
        let a = ["fdkaacenc", "avenc_aac", "voaacenc", "opusenc"].into_iter().find(|f| have(f)).map(String::from);
        (v, a)
    }

    fn use_va(ask: &Ask) -> bool {
        match std::env::var("HEARTH_CAPTURE_DECODER").as_deref() {
            Ok("cpu") => return false,
            Ok("va") => return have("vapostproc"),
            _ => {}
        }
        if VA_FAILED.load(Ordering::Relaxed) || !have("vapostproc") {
            return false;
        }
        let mjpg = ask.format == "MJPG" || ask.card.fake_picture.is_some() || ask.watching;
        // MJPEG needs vajpegdec; YUYV only the converter.
        !mjpg || have("vajpegdec")
    }

    pub fn running() -> bool {
        RUN.lock().unwrap().is_some()
    }

    /// Held while a pipeline is being taken down or set up, so a new one never meets the old one at the card.
    static TURN: Mutex<()> = Mutex::new(());

    pub fn stop() {
        GEN.fetch_add(1, Ordering::Relaxed);
        // A frame the window was told of but never took must not hold up the next pipeline's.
        PENDING.store(false, Ordering::Release);
        let r = RUN.lock().unwrap().take();
        *FRAME.lock().unwrap() = None;
        if let Some(r) = r {
            if let Some(b) = r.pipeline.bus() {
                let _ = b.post(gst::message::Application::new(gst::Structure::new_empty("hearth-stop")));
            }
            // Taken down off the window's thread (closing the card can take a moment).
            let _ = std::thread::Builder::new().name("cc-stop".into()).spawn(move || {
                let _turn = TURN.lock().unwrap();
                if let Some(rec) = r.rec {
                    // Stopping with a recording open: its file is closed properly first.
                    finish_rec(&r.pipeline, rec, r.gen, true);
                }
                let _ = r.pipeline.set_state(gst::State::Null);
            });
        }
    }

    pub fn set_volume(v: u32, muted: bool) {
        if let Some(r) = RUN.lock().unwrap().as_ref() {
            if let Some(vol) = r.pipeline.by_name("vol") {
                vol.set_property("volume", v.min(100) as f64 / 100.0);
                vol.set_property("mute", muted);
            }
        }
    }

    pub fn start(ask: Ask) {
        stop();
        let gen = GEN.fetch_add(1, Ordering::Relaxed) + 1;
        // Set up off the window's thread (opening the card and the first moments take a little while).
        let _ = std::thread::Builder::new().name("cc-start".into()).spawn(move || {
            let _turn = TURN.lock().unwrap();
            if gen != current() {
                return;
            }
            if !init() {
                send(gen, Event::Failed("GStreamer couldn't start on this PC.".into()));
                return;
            }
            let va = use_va(&ask);
            let r = match build(&ask, va, gen) {
                Ok(r) => Ok(r),
                Err(e) if va => {
                    // The graphics card's decoder isn't usable here: the CPU's, for as long as the app is open.
                    VA_FAILED.store(true, Ordering::Relaxed);
                    crate::log(&format!("the graphics card's decoder didn't start ({e}); decoding with the CPU"));
                    build(&ask, false, gen)
                }
                Err(e) => Err(e),
            };
            match r {
                Ok(r) => {
                    let mut g = RUN.lock().unwrap();
                    if gen == current() {
                        *g = Some(r);
                    } else {
                        let _ = r.pipeline.set_state(gst::State::Null);
                    }
                }
                Err(e) => send(gen, Event::Failed(e)),
            }
        });
    }

    fn build(ask: &Ask, va: bool, gen: u64) -> Result<Running, String> {
        let desc = describe(ask, va, have("pipewiresrc"));
        crate::log(&format!("pipeline: {desc}"));
        let el = gst::parse::launch(&desc).map_err(|e| format!("The player couldn't be set up: {e}"))?;
        let pipeline = el.downcast::<gst::Pipeline>().map_err(|_| "The player couldn't be set up.".to_string())?;
        let sink = pipeline.by_name("pic").and_then(|e| e.downcast::<gstreamer_app::AppSink>().ok()).ok_or("The player has no picture.")?;
        let (beat_tx, beat_rx) = std::sync::mpsc::channel::<()>();
        let watching = ask.watching;
        // The frames: into FRAME for the window; blank / real picture for the signal.
        {
            let started = Arc::new(AtomicBool::new(false));
            let seen_real = Arc::new(AtomicBool::new(false));
            let blank_since: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
            let signal_sent: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
            let n = Arc::new(AtomicU64::new(0));
            let fmt = if watching { ask.card.lightest().map(|m| m.0).unwrap_or_default() } else { ask.format.clone() };
            let (w0, h0, f0) = if watching { ask.card.lightest().map(|m| (m.1, m.2, m.3)).unwrap_or((ask.w, ask.h, ask.fps)) } else { (ask.w, ask.h, ask.fps) };
            let decoder = if va { "graphics card".to_string() } else { "CPU".to_string() };
            sink.set_callbacks(
                gstreamer_app::AppSinkCallbacks::builder()
                    .new_sample(move |s| {
                        let sample = s.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                        let _ = beat_tx.send(());
                        let (w, h) = sample.caps().and_then(|c| c.structure(0).map(|st| (st.get::<i32>("width").unwrap_or(0), st.get::<i32>("height").unwrap_or(0)))).unwrap_or((0, 0));
                        let Some(buf) = sample.buffer() else { return Ok(gst::FlowSuccess::Ok) };
                        let Ok(map) = buf.map_readable() else { return Ok(gst::FlowSuccess::Ok) };
                        let need = (w.max(0) * h.max(0) * 4) as usize;
                        if w <= 0 || h <= 0 || map.len() < need {
                            return Ok(gst::FlowSuccess::Ok);
                        }
                        let (w, h) = (w as u32, h as u32);
                        let k = n.fetch_add(1, Ordering::Relaxed);
                        if !started.swap(true, Ordering::Relaxed) {
                            send(gen, Event::Started { format: fmt.clone(), w: w0, h: h0, fps: f0, decoder: decoder.clone(), watching });
                        }
                        // The signal: every frame while waiting for one, every 6th otherwise.
                        if watching || k % 6 == 0 {
                            let b = blank(&map[..need], w, h);
                            let mut since = blank_since.lock().unwrap();
                            let mut sent = signal_sent.lock().unwrap();
                            if !b {
                                *since = None;
                                seen_real.store(true, Ordering::Relaxed);
                                if *sent != Some(true) {
                                    *sent = Some(true);
                                    send(gen, Event::Signal(true));
                                }
                            } else if !watching {
                                let t = *since.get_or_insert_with(Instant::now);
                                let limit = if seen_real.load(Ordering::Relaxed) { 10_000 } else { 1_500 };
                                if t.elapsed().as_millis() as u64 >= limit && *sent != Some(false) {
                                    *sent = Some(false);
                                    send(gen, Event::Signal(false));
                                }
                            }
                        }
                        if watching {
                            // Waiting for a signal: nothing is shown.
                            return Ok(gst::FlowSuccess::Ok);
                        }
                        let delay = match (s.current_running_time(), buf.pts()) {
                            (Some(now), Some(pts)) if now >= pts => (now - pts).nseconds(),
                            _ => 0,
                        };
                        *FRAME.lock().unwrap() = Some(Frame { w, h, rgba: map[..need].to_vec(), delay_ns: delay, at: Instant::now() });
                        if !PENDING.swap(true, Ordering::AcqRel) {
                            send(gen, Event::Frame);
                        }
                        Ok(gst::FlowSuccess::Ok)
                    })
                    .build(),
            );
        }
        // No frames at all for 2 s: no signal. Then it sleeps until a frame comes (or the pipeline goes).
        if !watching {
            let _ = std::thread::Builder::new().name("cc-beat".into()).stack_size(64 * 1024).spawn(move || {
                let mut said = false;
                loop {
                    let r = if said { beat_rx.recv().map_err(|_| std::sync::mpsc::RecvTimeoutError::Disconnected) } else { beat_rx.recv_timeout(Duration::from_secs(2)) };
                    match r {
                        Ok(()) => said = false,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            if gen != current() {
                                return;
                            }
                            said = true;
                            send(gen, Event::Signal(false));
                        }
                        Err(_) => return,
                    }
                }
            });
        } else {
            drop(beat_rx);
        }
        // Errors and the end, on the bus (a thread that sleeps there).
        let bus = pipeline.bus().ok_or("The player has no bus.")?;
        pipeline.set_state(gst::State::Playing).map_err(|_| first_error(&bus).unwrap_or_else(|| "The card couldn't be opened.".into()))?;
        // An error in the first moments (the VA decoder refusing the card's JPEGs): start again without it.
        if va {
            if let Some(m) = bus.timed_pop_filtered(gst::ClockTime::from_mseconds(1200), &[gst::MessageType::Error]) {
                let e = words(&m);
                let _ = pipeline.set_state(gst::State::Null);
                return Err(e);
            }
        }
        // CI's stand-in card: its JPEG, sent at the mode's frame rate (read again when the file changes).
        if let Some(pic) = ask.card.fake_picture.clone() {
            if let Some(src) = pipeline.by_name("src").and_then(|e| e.downcast::<gstreamer_app::AppSrc>().ok()) {
                let fps = if watching { ask.card.lightest().map(|m| m.3).unwrap_or(5) } else { ask.fps }.max(1);
                let _ = std::thread::Builder::new().name("cc-stand-in".into()).spawn(move || {
                    let step = Duration::from_micros(1_000_000 / fps as u64);
                    let (mut seen, mut data) = (None, Vec::new());
                    let mut next = Instant::now();
                    while gen == current() {
                        let m = std::fs::metadata(&pic).and_then(|m| m.modified()).ok();
                        if m != seen {
                            if let Ok(b) = std::fs::read(&pic) {
                                data = b;
                            }
                            seen = m;
                        }
                        if !data.is_empty() && src.push_buffer(gst::Buffer::from_slice(data.clone())).is_err() {
                            break;
                        }
                        next += step;
                        let now = Instant::now();
                        if next > now {
                            std::thread::sleep(next - now);
                        } else {
                            next = now;
                        }
                    }
                });
            }
        }
        let p2 = pipeline.clone();
        let _ = std::thread::Builder::new().name("cc-bus".into()).stack_size(128 * 1024).spawn(move || loop {
            let Some(m) = bus.timed_pop_filtered(gst::ClockTime::NONE, &[gst::MessageType::Error, gst::MessageType::Eos, gst::MessageType::Application]) else { return };
            match m.view() {
                gst::MessageView::Application(a) if a.structure().map(|s| s.name().as_str() == "hearth-stop").unwrap_or(false) => return,
                gst::MessageView::Error(_) => {
                    let e = words(&m);
                    crate::log(&format!("stopped: {e}"));
                    let mine = RUN.lock().unwrap().as_ref().map(|r| r.gen == gen).unwrap_or(false);
                    if mine {
                        let _ = p2.set_state(gst::State::Null);
                        *RUN.lock().unwrap() = None;
                        send(gen, Event::Failed(e));
                    }
                    return;
                }
                gst::MessageView::Eos(_) => {
                    let mine = RUN.lock().unwrap().as_ref().map(|r| r.gen == gen).unwrap_or(false);
                    if mine {
                        let _ = p2.set_state(gst::State::Null);
                        *RUN.lock().unwrap() = None;
                        send(gen, Event::Failed("The card stopped sending.".into()));
                    }
                    return;
                }
                _ => {}
            }
        });
        Ok(Running { gen, pipeline, va, rec: None })
    }

    fn first_error(bus: &gst::Bus) -> Option<String> {
        bus.timed_pop_filtered(gst::ClockTime::from_mseconds(300), &[gst::MessageType::Error]).map(|m| words(&m))
    }

    /// An error in plain words.
    fn words(m: &gst::Message) -> String {
        if let gst::MessageView::Error(e) = m.view() {
            let raw = e.error().to_string();
            let l = raw.to_lowercase();
            let from = m.src().map(|s| s.name().to_string()).unwrap_or_default();
            crate::log(&format!("GStreamer: {raw} ({from}) {:?}", e.debug()));
            if from == "src" && (l.contains("no such device") || l.contains("disconnected") || l.contains("could not read") || l.contains("failed to")) {
                return "The capture card was unplugged or stopped answering.".into();
            }
            if from == "src" && (l.contains("busy") || l.contains("resource busy")) {
                return "Another app is using the capture card. Close it, and the picture comes up here.".into();
            }
            if from == "src" && l.contains("not negotiate") {
                return "The card can't send that picture mode. Pick another in Settings.".into();
            }
            if from == "asrc" || from == "asink" {
                return format!("The sound couldn't start: {raw}");
            }
            return format!("The picture stopped: {raw}");
        }
        "The picture stopped.".into()
    }

    pub fn recording() -> Option<(Instant, PathBuf)> {
        RUN.lock().unwrap().as_ref().and_then(|r| r.rec.as_ref().map(|x| (x.since, x.file.clone())))
    }

    /// Starts recording into `file` (the window hears Recording(Ok) or Recording(Err)).
    pub fn record_start(file: PathBuf) {
        let mut g = RUN.lock().unwrap();
        let Some(r) = g.as_mut() else {
            send(current(), Event::Recording(Err("There's no picture to record yet.".into())));
            return;
        };
        let gen = r.gen;
        if r.rec.is_some() {
            return;
        }
        let (venc, aenc) = encoders();
        let Some(venc) = venc else {
            send(gen, Event::Recording(Err("This PC can't record video: it has no H.264 encoder for GStreamer (gstreamer1-plugin-openh264 or the graphics card's VA-API).".into())));
            return;
        };
        let at = r.pipeline.by_name("at");
        let desc = describe_rec(&file, r.va, &venc, at.as_ref().and(aenc.as_deref()));
        crate::log(&format!("recording: {desc}"));
        if let Some(d) = file.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let bin = match gst::parse::bin_from_description(&desc, false) {
            Ok(b) => b,
            Err(e) => {
                crate::log(&format!("recording couldn't be set up: {e}"));
                send(gen, Event::Recording(Err(format!("Recording couldn't start: {e}"))));
                return;
            }
        };
        let mut pads = Vec::new();
        let mut why: Vec<String> = Vec::new();
        let mut ok = r.pipeline.add(&bin).is_ok();
        if !ok {
            why.push("add".into());
        }
        for (tee_name, q) in [("vt", "rvq"), ("at", "raq")] {
            let (Some(tee), Some(qe)) = (r.pipeline.by_name(tee_name), bin.by_name(q)) else { continue };
            let Some(sinkpad) = qe.static_pad("sink") else { continue };
            let ghost = match gst::GhostPad::builder_with_target(&sinkpad) {
                Ok(b) => b.name(format!("in_{q}")).build(),
                Err(e) => {
                    why.push(format!("ghost {q}: {e}"));
                    ok = false;
                    continue;
                }
            };
            let ghost = ghost.upcast::<gst::Pad>();
            let _ = ghost.set_active(true);
            if let Err(e) = bin.add_pad(&ghost) {
                why.push(format!("add pad {q}: {e}"));
                ok = false;
            }
            match tee.request_pad_simple("src_%u") {
                Some(tp) => {
                    if let Err(e) = tp.link(&ghost) {
                        why.push(format!("link {tee_name}: {e:?}"));
                        ok = false;
                    }
                    pads.push((tee, tp));
                }
                None => {
                    why.push(format!("no pad on {tee_name}"));
                    ok = false;
                }
            }
        }
        if ok {
            if let Err(e) = bin.sync_state_with_parent() {
                why.push(format!("start: {e}"));
                ok = false;
            }
        }
        if !ok {
            crate::log(&format!("recording: hooking it in failed: {}", why.join("; ")));
        }
        if !ok {
            crate::log("recording couldn't be hooked into the picture");
            for (tee, tp) in pads {
                tee.release_request_pad(&tp);
            }
            let _ = bin.set_state(gst::State::Null);
            let _ = r.pipeline.remove(&bin);
            send(gen, Event::Recording(Err("Recording couldn't start.".into())));
            return;
        }
        crate::log(&format!("recording into {}", file.display()));
        r.rec = Some(Rec { bin, pads, file: file.clone(), since: Instant::now() });
        send(gen, Event::Recording(Ok(file)));
    }

    /// Stops recording: the file is finished (its end written) and closed; the picture goes on.
    pub fn record_stop() {
        let mut g = RUN.lock().unwrap();
        let Some(r) = g.as_mut() else { return };
        let Some(rec) = r.rec.take() else { return };
        let (p, gen) = (r.pipeline.clone(), r.gen);
        drop(g);
        finish_rec(&p, rec, gen, false);
    }

    /// Unhooks the recording from the tees, ends its stream (so mp4mux writes the file's end), waits for it to reach
    /// the file (at most 5 s), then takes it out of the pipeline.
    fn finish_rec(p: &gst::Pipeline, rec: Rec, gen: u64, blocking: bool) {
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        if let Some(fs) = rec.bin.by_name("rfile").and_then(|f| f.static_pad("sink")) {
            let tx = tx.clone();
            fs.add_probe(gst::PadProbeType::EVENT_DOWNSTREAM, move |_, info| {
                if let Some(gst::PadProbeData::Event(ev)) = &info.data {
                    if ev.type_() == gst::EventType::Eos {
                        let _ = tx.send(());
                    }
                }
                gst::PadProbeReturn::Ok
            });
        }
        for (_tee, tp) in &rec.pads {
            let Some(peer) = tp.peer() else { continue };
            tp.add_probe(gst::PadProbeType::IDLE, move |pad, _| {
                let _ = pad.unlink(&peer);
                let _ = peer.send_event(gst::event::Eos::new());
                gst::PadProbeReturn::Remove
            });
        }
        let p = p.clone();
        let file = rec.file.clone();
        let done = move || {
            let ok = rx.recv_timeout(Duration::from_secs(5)).is_ok();
            let _ = rec.bin.set_state(gst::State::Null);
            let _ = p.remove(&rec.bin);
            for (tee, tp) in &rec.pads {
                tee.release_request_pad(tp);
            }
            let good = ok && std::fs::metadata(&file).map(|m| m.len() > 0).unwrap_or(false);
            crate::log(&format!("recording ended (its end reached the file: {ok}; {} bytes)", std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0)));
            send(gen, Event::Recorded(if good { Ok(file.clone()) } else { Err(format!("The recording {} may be cut short.", file.file_name().unwrap_or_default().to_string_lossy())) }));
        };
        if blocking {
            done();
        } else {
            let _ = std::thread::Builder::new().name("cc-rec-end".into()).spawn(done);
        }
    }

    /// A recording's length, picture and a thumbnail (480 × 270) from its first second.
    pub fn probe(path: &Path) -> Option<Probe> {
        if !init() {
            return None;
        }
        let desc = format!(
            "filesrc location=\"{}\" ! decodebin ! videoconvert name=cv ! videoscale ! video/x-raw,format=RGBA,width=480,height=270,pixel-aspect-ratio=1/1 ! appsink name=t sync=false max-buffers=1",
            path.display().to_string().replace('"', "")
        );
        let p = gst::parse::launch(&desc).ok()?.downcast::<gst::Pipeline>().ok()?;
        let sink = p.by_name("t")?.downcast::<gstreamer_app::AppSink>().ok()?;
        let _ = p.set_state(gst::State::Paused);
        let (res, _, _) = p.state(gst::ClockTime::from_seconds(8));
        if res.is_err() {
            let _ = p.set_state(gst::State::Null);
            return None;
        }
        let secs = p.query_duration::<gst::ClockTime>().map(|d| d.seconds()).unwrap_or(0);
        if secs > 2 {
            let _ = p.seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, gst::ClockTime::from_seconds(1));
            let _ = p.state(gst::ClockTime::from_seconds(5));
        }
        let sample = sink.try_pull_preroll(gst::ClockTime::from_seconds(5));
        let (mut w, mut h, mut fps) = (0, 0, 0);
        if let Some(c) = p.by_name("cv").and_then(|e| e.static_pad("sink")).and_then(|pd| pd.current_caps()) {
            if let Some(s) = c.structure(0) {
                w = s.get::<i32>("width").unwrap_or(0).max(0) as u32;
                h = s.get::<i32>("height").unwrap_or(0).max(0) as u32;
                if let Ok(f) = s.get::<gst::Fraction>("framerate") {
                    if f.denom() > 0 {
                        fps = (f.numer() as f64 / f.denom() as f64).round() as u32;
                    }
                }
            }
        }
        let rgba = sample.as_ref().and_then(|s| s.buffer().and_then(|b| b.map_readable().ok().map(|m| m.to_vec()))).unwrap_or_default();
        let _ = p.set_state(gst::State::Null);
        if rgba.len() < 480 * 270 * 4 {
            return None;
        }
        Some(Probe { secs, w, h, fps, tw: 480, th: 270, rgba: rgba[..480 * 270 * 4].to_vec() })
    }
}

pub use imp::{encoders, probe, record_start, record_stop, recording, running, set_volume, start, stop};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Mode;

    fn ask() -> Ask {
        Ask {
            card: Card {
                dev: "/dev/video2".into(),
                id: String::new(),
                name: "Monster 4K".into(),
                chip: String::new(),
                speed: 5000,
                usb_port: String::new(),
                modes: vec![Mode { format: "MJPG".into(), w: 1920, h: 1080, fps: vec![60, 30] }, Mode { format: "MJPG".into(), w: 640, h: 480, fps: vec![30, 5] }],
                fake_picture: None,
                fake_sound: None,
            },
            format: "MJPG".into(),
            w: 1920,
            h: 1080,
            fps: 60,
            low_delay: true,
            sound: Some("alsa_input.usb-X".into()),
            play_sound: true,
            volume: 70,
            muted: false,
            watching: false,
        }
    }

    #[test]
    fn the_pipelines() {
        let d = describe(&ask(), true, true);
        assert!(d.starts_with("v4l2src name=src device=\"/dev/video2\" ! image/jpeg,width=1920,height=1080,framerate=60/1 ! vajpegdec"));
        assert!(d.contains("appsink name=pic sync=false max-buffers=1 drop=true"), "Low delay: no frame waits");
        assert!(d.contains("pipewiresrc name=asrc target-object=\"alsa_input.usb-X\""));
        assert!(d.contains("volume name=vol volume=0.70 mute=false ! pipewiresink"));
        let cpu = describe(&Ask { low_delay: false, play_sound: false, ..ask() }, false, true);
        assert!(cpu.contains("jpegdec name=dec") && cpu.contains("sync=true max-buffers=2"));
        assert!(!cpu.contains("pipewiresink"), "Play the Xbox's sound off: nothing plays");
        let y = describe(&Ask { format: "YUYV".into(), ..ask() }, false, false);
        assert!(y.contains("video/x-raw,format=YUY2") && y.contains("pulsesrc"));
        // Waiting for a signal: the lightest mode, no sound.
        let w = describe(&Ask { watching: true, ..ask() }, false, true);
        assert!(w.contains("width=640,height=480,framerate=5/1") && !w.contains("asrc"));
        // CI's stand-in card: its JPEG as the card's MJPEG, the rest the same.
        let mut a = ask();
        a.card.fake_picture = Some("/tmp/scene.jpg".into());
        let f = describe(&a, false, false);
        assert!(f.starts_with("appsrc name=src is-live=true do-timestamp=true format=time caps=\"image/jpeg,framerate=60/1\" ! jpegparse"));
        assert!(f.contains("audiotestsrc") && f.contains("jpegdec name=dec"));
        let r = describe_rec(Path::new("/v/Capture 1.mp4"), true, "vah264enc", Some("fdkaacenc"));
        assert!(r.contains("vapostproc ! video/x-raw,format=NV12") && r.contains("vah264enc bitrate=30000") && r.contains("mp4mux name=mux") && r.contains("fdkaacenc"));
        assert!(describe_rec(Path::new("/v/a.mp4"), false, "openh264enc", None).contains("openh264enc") );
    }

    #[test]
    fn a_blank_picture_is_no_signal() {
        let (w, h) = (64u32, 36u32);
        let black = vec![0u8; (w * h * 4) as usize];
        assert!(blank(&black, w, h));
        let mut blue = black.clone();
        for p in blue.chunks_mut(4) {
            p[2] = 200;
        }
        assert!(blank(&blue, w, h), "a one-colour 'no signal' screen");
        let mut pic = black.clone();
        for (i, p) in pic.chunks_mut(4).enumerate() {
            p[0] = (i % 251) as u8;
            p[1] = (i * 7 % 253) as u8;
        }
        assert!(!blank(&pic, w, h));
        assert!(blank(&[], 0, 0));
    }
}
