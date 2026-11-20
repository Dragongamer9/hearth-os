//! The stream (docs/CONSOLE-REMOTE-PLAY.md, "The stream", "The stack"): one worker thread that does the steps in
//! order (wake, play, wait for the Xbox, the WebRTC offer and answer, ICE, the channels), and GStreamer's webrtcbin for
//! the picture and sound: the picture decoded on the graphics card (VA-API's vah264dec through decodebin, a CPU
//! decoder only when there's none), scaled to the screen there (vapostproc) and handed over one frame at a time (late
//! frames dropped, the jitter buffer at its smallest); the sound to PipeWire. Nothing of this runs until Stream is
//! pressed, and it all stops at Disconnect.
//!
//! The window hears from it through `events()` (steps, the first picture, stats once a second, data used, the end),
//! and takes frames from `frame()` when told one is ready.

use crate::input::{self, Pad};
use crate::xal::Stream as Tok;
use crate::xhome::{self, Kind, Session, State};
use crate::{chan, spad};
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Waking,
    Woke(u64),
    Connecting,
    Picture,
    Queue(u64),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stats {
    pub delay_ms: Option<u32>,
    pub mbps: f32,
    pub height: u32,
    pub fps: u32,
    pub loss: f32,
    pub session_bytes: u64,
}

impl Stats {
    /// Good / Fair / Poor, and how many of the four bars.
    pub fn quality(&self) -> (&'static str, u8) {
        let d = self.delay_ms.unwrap_or(0);
        if self.loss < 0.01 && d < 60 {
            ("Good", if self.height >= 1080 { 4 } else { 3 })
        } else if self.loss < 0.03 && d < 120 {
            ("Fair", 2)
        } else {
            ("Poor", 1)
        }
    }
    pub fn resolution(&self) -> String {
        if self.height == 0 {
            return "…".into();
        }
        format!("{}p{}", self.height, if self.fps >= 50 { 60 } else { 30 })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Step(Step),
    /// The first picture: streaming.
    Started,
    Frame,
    Stats(Stats),
    /// Bytes used since the last one (for the monthly count; once a minute and at the end).
    Data(u64),
    /// It couldn't connect (what happened, in plain words, and whether the Xbox woke).
    Failed { why: String, woke: bool },
    /// The controller's presses for the menu (View + Menu held, then the menu's buttons).
    Nav(spad::Nav),
    Ended,
}

pub struct Frame {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

/// One channel for the app's whole life (the window's one subscription waits on it, using nothing at rest); each
/// stream's events carry its number, so an old stream's last words are told apart from a new one's.
static EVENTS: OnceLock<(UnboundedSender<(u64, Event)>, Mutex<Option<UnboundedReceiver<(u64, Event)>>>)> = OnceLock::new();
static GEN: AtomicU64 = AtomicU64::new(0);

fn bus() -> &'static (UnboundedSender<(u64, Event)>, Mutex<Option<UnboundedReceiver<(u64, Event)>>>) {
    EVENTS.get_or_init(|| {
        let (tx, rx) = unbounded();
        (tx, Mutex::new(Some(rx)))
    })
}

/// A stream's way to tell the window.
#[derive(Clone)]
pub struct Tx {
    gen: u64,
}

impl Tx {
    fn new() -> Tx {
        Tx { gen: GEN.fetch_add(1, Ordering::Relaxed) + 1 }
    }
    pub fn send(&self, e: Event) {
        let _ = bus().0.unbounded_send((self.gen, e));
    }
}

/// The number of the stream started last (the window ignores events from others).
pub fn current() -> u64 {
    GEN.load(Ordering::Relaxed)
}
static FRAME: Mutex<Option<Frame>> = Mutex::new(None);
static FRAME_PENDING: AtomicBool = AtomicBool::new(false);
static RUNNING: Mutex<Option<Arc<Control>>> = Mutex::new(None);

/// The window's subscription: the running stream's events (nothing when none runs).
pub fn events() -> impl futures::Stream<Item = (u64, Event)> {
    let rx = bus().1.lock().unwrap().take();
    match rx {
        Some(r) => futures::stream::StreamExt::boxed(r),
        None => futures::stream::StreamExt::boxed(futures::stream::pending()),
    }
}

/// The newest frame (the window shows it, then asks again on the next Frame event).
pub fn frame() -> Option<Frame> {
    FRAME_PENDING.store(false, Ordering::Release);
    FRAME.lock().unwrap().take()
}

/// The last frame shown, kept for Screenshot.
pub static LAST: Mutex<Option<(u32, u32, Vec<u8>)>> = Mutex::new(None);

struct Control {
    stop: AtomicBool,
    volume: AtomicU32,
    flags: Arc<spad::Flags>,
}

#[derive(Debug, Clone)]
pub struct Ask {
    pub kind: Kind,
    pub token: Tok,
    /// The console (Home) or the title (Cloud).
    pub target: String,
    /// Wake first (and the xccs token to do it with).
    pub wake: Option<crate::xal::Xsts>,
    pub hd: bool,
    /// Data saver: a bitrate cap (kbps), 0 for none.
    pub cap_kbps: u32,
    /// The picture's size on this screen.
    pub out: (u32, u32),
    pub volume: u32,
    pub sound: String,
    pub vibration: bool,
    pub prefer_builtin: bool,
    /// For Cloud's connect step.
    pub transfer: Option<String>,
}

pub fn running() -> bool {
    RUNNING.lock().unwrap().is_some()
}

pub fn stop() {
    if let Some(c) = RUNNING.lock().unwrap().as_ref() {
        c.stop.store(true, Ordering::Relaxed);
        c.flags.stop.store(true, Ordering::Relaxed);
    }
}

pub fn set_volume(v: u32) {
    if let Some(c) = RUNNING.lock().unwrap().as_ref() {
        c.volume.store(v.min(100), Ordering::Relaxed);
    }
}

pub fn set_menu_open(open: bool) {
    if let Some(c) = RUNNING.lock().unwrap().as_ref() {
        c.flags.menu_open.store(open, Ordering::Relaxed);
    }
}

pub fn set_vibration(on: bool) {
    if let Some(c) = RUNNING.lock().unwrap().as_ref() {
        c.flags.vibration.store(on, Ordering::Relaxed);
    }
}


/// Starts a stream (one at a time).
pub fn start(ask: Ask) {
    stop();
    let tx = Tx::new();
    let flags = Arc::new(spad::Flags::default());
    flags.vibration.store(ask.vibration, Ordering::Relaxed);
    let ctl = Arc::new(Control { stop: AtomicBool::new(false), volume: AtomicU32::new(ask.volume), flags });
    *RUNNING.lock().unwrap() = Some(ctl.clone());
    let _ = std::thread::Builder::new().name("crp-stream".into()).spawn(move || {
        let out = run(&ask, &ctl, &tx);
        match out {
            Ok(()) => {}
            Err((why, woke)) => {
                tx.send(Event::Failed { why, woke });
            }
        }
        tx.send(Event::Ended);
        let mut g = RUNNING.lock().unwrap();
        if g.as_ref().map(|c| Arc::ptr_eq(c, &ctl)).unwrap_or(false) {
            *g = None;
        }
    });
}

fn stopped(ctl: &Control) -> bool {
    ctl.stop.load(Ordering::Relaxed)
}

/// Waits `ms`, waking early when stopped.
fn nap(ctl: &Control, ms: u64) -> bool {
    let t = Instant::now();
    while t.elapsed() < Duration::from_millis(ms) {
        if stopped(ctl) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    !stopped(ctl)
}

type Out = Result<(), (String, bool)>;

fn run(ask: &Ask, ctl: &Arc<Control>, tx: &Tx) -> Out {
    let send = |e: Event| tx.send(e);
    let mut woke = false;
    // 1. Wake (Home, when it's asleep): asked again after 5, 10, 15 … s, for at most 45 s.
    if let Some(w) = &ask.wake {
        send(Event::Step(Step::Waking));
        let t = Instant::now();
        xhome::wake(w, &ask.target).map_err(|e| (format!("Your Xbox couldn't be woken: {}", e.words()), false))?;
        loop {
            if !nap(ctl, 5000) {
                return Ok(());
            }
            if xhome::power(w, &ask.target).map(|p| p == xhome::Power::On).unwrap_or(false) {
                break;
            }
            if t.elapsed() > Duration::from_secs(45) {
                return Err(("Your Xbox didn't wake up. Is it plugged in, with Power options set to Sleep?".into(), false));
            }
        }
        woke = true;
        send(Event::Step(Step::Woke(t.elapsed().as_secs())));
    }
    // 2. The session, and waiting for the Xbox (or Cloud's queue).
    send(Event::Step(Step::Connecting));
    let ses = xhome::play(&ask.token, ask.kind, &ask.target, ask.hd).map_err(|e| (format!("The stream couldn't start: {}", e.words()), woke))?;
    let t = Instant::now();
    let mut connected = false;
    loop {
        if stopped(ctl) {
            ses.stop();
            return Ok(());
        }
        match ses.state().map_err(|e| (e.words(), woke))? {
            State::Provisioned => break,
            State::ReadyToConnect if !connected => {
                connected = true;
                if let Some(tok) = &ask.transfer {
                    let _ = ses.connect(tok);
                }
            }
            State::Waiting(s) => send(Event::Step(Step::Queue(s))),
            State::Failed(m) => return Err((format!("Your Xbox said: {m}"), woke)),
            _ => {}
        }
        if t.elapsed() > Duration::from_secs(if ask.kind == Kind::Cloud { 600 } else { 90 }) {
            ses.stop();
            return Err(("Your Xbox didn't answer in time.".into(), woke));
        }
        if !nap(ctl, 2000) {
            ses.stop();
            return Ok(());
        }
    }
    let _ = ses.configuration();
    let r = media(ask, ctl, tx, &ses, woke);
    ses.stop();
    r
}

#[cfg(not(feature = "stream"))]
fn media(_ask: &Ask, _ctl: &Arc<Control>, _tx: &Tx, _ses: &Session, woke: bool) -> Out {
    Err(("This copy of Hearth Console Remote Play was built without its player (GStreamer).".into(), woke))
}

#[cfg(feature = "stream")]
fn media(ask: &Ask, ctl: &Arc<Control>, tx: &Tx, ses: &Session, woke: bool) -> Out {
    use gstreamer as gst;
    use gstreamer::prelude::*;
    use gstreamer_webrtc as gwr;

    let fail = |s: &str| (s.to_string(), woke);
    gst::init().map_err(|_| fail("GStreamer couldn't start on this device."))?;
    let pipeline = gst::Pipeline::new();
    let webrtc = gst::ElementFactory::make("webrtcbin").name("crp").build().map_err(|_| fail("This device has no WebRTC player (GStreamer's webrtcbin)."))?;
    webrtc.set_property_from_str("bundle-policy", "max-bundle");
    webrtc.set_property_from_str("stun-server", "stun://stun.l.google.com:19302");
    // The jitter buffer at its smallest: the picture as soon as it's here.
    webrtc.set_property("latency", 0u32);
    pipeline.add(&webrtc).map_err(|_| fail("The player couldn't start."))?;

    let audio_caps = gst::Caps::builder("application/x-rtp").field("media", "audio").field("encoding-name", "OPUS").field("payload", 111i32).field("clock-rate", 48000i32).build();
    let video_caps = gst::Caps::builder("application/x-rtp").field("media", "video").field("encoding-name", "H264").field("payload", 102i32).field("clock-rate", 90000i32).build();
    let _a: gwr::WebRTCRTPTransceiver = webrtc.emit_by_name("add-transceiver", &[&gwr::WebRTCRTPTransceiverDirection::Sendrecv, &audio_caps]);
    let _v: gwr::WebRTCRTPTransceiver = webrtc.emit_by_name("add-transceiver", &[&gwr::WebRTCRTPTransceiverDirection::Recvonly, &video_caps]);

    // The picture and sound, when their streams arrive.
    let (out_w, out_h) = ask.out;
    let volume_el: Arc<Mutex<Option<gst::Element>>> = Arc::new(Mutex::new(None));
    let decoded = Arc::new(AtomicU64::new(0));
    let height = Arc::new(AtomicU32::new(0));
    let started = Arc::new(AtomicBool::new(false));
    {
        let pipeline = pipeline.clone();
        let volume_el = volume_el.clone();
        let decoded = decoded.clone();
        let height = height.clone();
        let started = started.clone();
        let tx = tx.clone();
        let sound = ask.sound.clone();
        let vol = ctl.volume.load(Ordering::Relaxed);
        webrtc.connect_pad_added(move |_, pad| {
            if pad.direction() != gst::PadDirection::Src {
                return;
            }
            let Ok(dec) = gst::ElementFactory::make("decodebin").build() else { return };
            let _ = pipeline.add(&dec);
            let _ = dec.sync_state_with_parent();
            if let Some(sink) = dec.static_pad("sink") {
                let _ = pad.link(&sink);
            }
            let pipeline = pipeline.clone();
            let volume_el = volume_el.clone();
            let decoded = decoded.clone();
            let height = height.clone();
            let started = started.clone();
            let tx = tx.clone();
            let sound = sound.clone();
            dec.connect_pad_added(move |_, dpad| {
                let caps = dpad.current_caps().or_else(|| Some(dpad.query_caps(None)));
                let name = caps.as_ref().and_then(|c| c.structure(0)).map(|s| s.name().to_string()).unwrap_or_default();
                if name.starts_with("video") {
                    if let Some(s) = caps.as_ref().and_then(|c| c.structure(0)) {
                        if let Ok(h) = s.get::<i32>("height") {
                            height.store(h.max(0) as u32, Ordering::Relaxed);
                        }
                    }
                    let scale = if gst::ElementFactory::find("vapostproc").is_some() { "vapostproc" } else { "videoconvertscale" };
                    let desc = format!("queue max-size-buffers=1 leaky=downstream ! {scale} ! video/x-raw,format=RGBA,width={out_w},height={out_h},pixel-aspect-ratio=1/1 ! appsink name=pic sync=false max-buffers=1 drop=true");
                    let Ok(bin) = gst::parse::bin_from_description(&desc, true) else { return };
                    let _ = pipeline.add(&bin);
                    let _ = bin.sync_state_with_parent();
                    if let Some(sink) = bin.static_pad("sink") {
                        let _ = dpad.link(&sink);
                    }
                    if let Some(app) = bin.by_name("pic").and_then(|e| e.downcast::<gstreamer_app::AppSink>().ok()) {
                        let decoded = decoded.clone();
                        let started = started.clone();
                        let tx = tx.clone();
                        app.set_callbacks(
                            gstreamer_app::AppSinkCallbacks::builder()
                                .new_sample(move |s| {
                                    let sample = s.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                                    let (w, h) = sample.caps().and_then(|c| c.structure(0).map(|st| (st.get::<i32>("width").unwrap_or(0), st.get::<i32>("height").unwrap_or(0)))).unwrap_or((0, 0));
                                    if let Some(buf) = sample.buffer() {
                                        if let Ok(map) = buf.map_readable() {
                                            if w > 0 && h > 0 && map.len() >= (w * h * 4) as usize {
                                                *FRAME.lock().unwrap() = Some(Frame { w: w as u32, h: h as u32, rgba: map[..(w * h * 4) as usize].to_vec() });
                                                decoded.fetch_add(1, Ordering::Relaxed);
                                                if !started.swap(true, Ordering::Relaxed) {
                                                    tx.send(Event::Started);
                                                }
                                                if !FRAME_PENDING.swap(true, Ordering::AcqRel) {
                                                    tx.send(Event::Frame);
                                                }
                                            }
                                        }
                                    }
                                    Ok(gst::FlowSuccess::Ok)
                                })
                                .build(),
                        );
                    }
                } else if name.starts_with("audio") {
                    let sink = if gst::ElementFactory::find("pipewiresink").is_some() {
                        if sound.is_empty() { "pipewiresink".to_string() } else { format!("pipewiresink target-object=\"{}\"", sound.replace('"', "")) }
                    } else {
                        "autoaudiosink".to_string()
                    };
                    let desc = format!("queue ! audioconvert ! audioresample ! volume name=vol volume={} ! {sink}", vol as f64 / 100.0);
                    let Ok(bin) = gst::parse::bin_from_description(&desc, true) else { return };
                    let _ = pipeline.add(&bin);
                    let _ = bin.sync_state_with_parent();
                    if let Some(sink) = bin.static_pad("sink") {
                        let _ = dpad.link(&sink);
                    }
                    *volume_el.lock().unwrap() = bin.by_name("vol");
                }
            });
        });
    }

    // ICE candidates as they're found (kept until the answer is set, then sent together).
    let local: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
    let mids: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    {
        let local = local.clone();
        let mids = mids.clone();
        webrtc.connect("on-ice-candidate", false, move |v| {
            let idx = v.get(1).and_then(|x| x.get::<u32>().ok()).unwrap_or(0);
            let cand = v.get(2).and_then(|x| x.get::<String>().ok()).unwrap_or_default();
            let mid = mids.lock().unwrap().get(idx as usize).cloned().unwrap_or_else(|| idx.to_string());
            if !cand.is_empty() {
                local.lock().unwrap().push(json!({"candidate": format!("a={}", cand.trim_start_matches("a=")), "sdpMid": mid, "sdpMLineIndex": idx}));
            }
            None
        });
    }

    pipeline.set_state(gst::State::Playing).map_err(|_| fail("The player couldn't start."))?;

    // The four data channels (before the offer, so it carries them).
    let (rumble_tx, rumble_rx) = mpsc::channel::<(f32, f32, u16)>();
    let mut channels: Vec<(String, gwr::WebRTCDataChannel)> = Vec::new();
    for (label, ordered, proto) in chan::CHANNELS {
        let opts = gst::Structure::builder("config").field("ordered", ordered).field("protocol", proto).build();
        let ch: Option<gwr::WebRTCDataChannel> = webrtc.emit_by_name("create-data-channel", &[&label, &opts]);
        match ch {
            Some(c) => channels.push((label.to_string(), c)),
            None => return Err(fail("The player couldn't open its channels.")),
        }
    }
    let by = |n: &str| channels.iter().find(|(l, _)| l == n).map(|(_, c)| c.clone());
    let (Some(input_ch), Some(control_ch), Some(message_ch)) = (by("input"), by("control"), by("message")) else { return Err(fail("The player couldn't open its channels.")) };
    let ready = Arc::new(AtomicBool::new(false));
    {
        let m2 = message_ch.clone();
        message_ch.connect_on_open(move |_| {
            m2.send_string(Some(&chan::handshake()));
        });
        let ready = ready.clone();
        let control_ch = control_ch.clone();
        let m3 = message_ch.clone();
        let (w, h) = if ask.hd { (1920, 1080) } else { (1280, 720) };
        message_ch.connect_on_message_string(move |_, s| {
            if s.map(chan::is_ack).unwrap_or(false) && !ready.swap(true, Ordering::Relaxed) {
                for m in chan::after_ack(w, h) {
                    m3.send_string(Some(&m));
                }
                for m in chan::control_start() {
                    control_ch.send_string(Some(&m));
                }
            }
        });
        let rumble_tx = rumble_tx.clone();
        input_ch.connect_on_message_data(move |_, d| {
            if let Some(r) = d.and_then(|b| input::rumble(b.as_ref())) {
                let _ = rumble_tx.send((r.strong, r.weak, r.ms));
            }
        });
    }

    // The offer.
    let (otx, orx) = mpsc::channel::<Option<String>>();
    {
        let promise = gst::Promise::with_change_func(move |reply| {
            let sdp = reply.ok().flatten().and_then(|r| r.value("offer").ok().and_then(|v| v.get::<gwr::WebRTCSessionDescription>().ok())).and_then(|o| o.sdp().as_text().ok());
            let _ = otx.send(sdp);
        });
        webrtc.emit_by_name::<()>("create-offer", &[&None::<gst::Structure>, &promise]);
    }
    let offer = orx.recv_timeout(Duration::from_secs(10)).ok().flatten().ok_or_else(|| fail("The player couldn't make its offer."))?;
    let offer = xhome::munge_offer(&offer, ask.cap_kbps);
    *mids.lock().unwrap() = offer.split("\r\n").filter_map(|l| l.strip_prefix("a=mid:")).map(String::from).collect();
    let sdp = gstreamer_sdp::SDPMessage::parse_buffer(offer.as_bytes()).map_err(|_| fail("The player's offer was malformed."))?;
    let desc = gwr::WebRTCSessionDescription::new(gwr::WebRTCSDPType::Offer, sdp);
    webrtc.emit_by_name::<()>("set-local-description", &[&desc, &None::<gst::Promise>]);
    ses.send_offer(&offer).map_err(|e| (format!("Your Xbox didn't take the connection: {}", e.words()), woke))?;
    let mut answer = None;
    for _ in 0..30 {
        if stopped(ctl) {
            let _ = pipeline.set_state(gst::State::Null);
            return Ok(());
        }
        match ses.answer() {
            Ok(Some(a)) => {
                answer = Some(a);
                break;
            }
            Ok(None) => {}
            Err(e) => return Err((e.words(), woke)),
        }
        nap(ctl, 1000);
    }
    let answer = answer.ok_or_else(|| fail("Your Xbox didn't answer the connection."))?;
    let sdp = gstreamer_sdp::SDPMessage::parse_buffer(answer.as_bytes()).map_err(|_| fail("Your Xbox's answer was malformed."))?;
    webrtc.emit_by_name::<()>("set-remote-description", &[&gwr::WebRTCSessionDescription::new(gwr::WebRTCSDPType::Answer, sdp), &None::<gst::Promise>]);

    // ICE: ours (after a moment to gather), then the Xbox's.
    nap(ctl, 1500);
    let ours = local.lock().unwrap().clone();
    ses.send_ice(&ours).map_err(|e| (e.words(), woke))?;
    for _ in 0..20 {
        match ses.remote_ice() {
            Ok(Some(list)) => {
                for c in list {
                    let cand = c["candidate"].as_str().unwrap_or("").trim_start_matches("a=").to_string();
                    let idx = c["sdpMLineIndex"].as_u64().unwrap_or(0) as u32;
                    if !cand.is_empty() {
                        webrtc.emit_by_name::<()>("add-ice-candidate", &[&idx, &cand]);
                    }
                }
                break;
            }
            Ok(None) => {}
            Err(_) => break,
        }
        if !nap(ctl, 1000) {
            break;
        }
    }

    // 3. The picture: the controller goes to the Xbox once the Xbox says hello.
    tx.send(Event::Step(Step::Picture));
    let (nav_tx, nav_rx) = mpsc::channel::<spad::Nav>();
    {
        // The menu's presses to the window (this thread ends with the controller's reader).
        let tx = tx.clone();
        let _ = std::thread::Builder::new().name("crp-nav".into()).spawn(move || {
            for n in nav_rx {
                tx.send(Event::Nav(n));
            }
        });
    }
    let seq = Arc::new(AtomicU32::new(1));
    let input_started = Arc::new(AtomicBool::new(false));
    {
        let ch = input_ch.clone();
        let seq = seq.clone();
        let ready = ready.clone();
        let input_started = input_started.clone();
        let t0 = Instant::now();
        let tx2 = tx.clone();
        let nav_tx2 = nav_tx.clone();
        spad::run(
            ask.prefer_builtin,
            ctl.flags.clone(),
            move |p: Pad| {
                if !ready.load(Ordering::Relaxed) {
                    return;
                }
                if !input_started.swap(true, Ordering::Relaxed) {
                    let b = input::client_metadata(0, 0.0, 1);
                    ch.send_data(Some(&gstreamer::glib::Bytes::from_owned(b)));
                }
                let n = seq.fetch_add(1, Ordering::Relaxed);
                let b = input::gamepad(n, t0.elapsed().as_secs_f64() * 1000.0, &[(0, p)]);
                ch.send_data(Some(&gstreamer::glib::Bytes::from_owned(b)));
            },
            nav_tx2,
            rumble_rx,
        );
        let _ = tx2;
    }
    // The menu's presses wake the window through the same events (a Frame-like nudge is enough).
    let _ = nav_tx;

    // Live: stats once a second, keepalive once a minute, data once a minute.
    let mut last_bytes = 0u64;
    let mut counted = 0u64;
    let mut last_t = Instant::now();
    let mut last_frames = 0u64;
    let mut ka = Instant::now();
    let mut minute = Instant::now();
    let begun = Instant::now();
    let mut vol_now = ctl.volume.load(Ordering::Relaxed);
    loop {
        if !nap(ctl, 1000) {
            break;
        }
        if !started.load(Ordering::Relaxed) && begun.elapsed() > Duration::from_secs(25) {
            let _ = pipeline.set_state(gst::State::Null);
            return Err(("The picture couldn't get through to this device in time.".into(), woke));
        }
        // Volume from the bar.
        let v = ctl.volume.load(Ordering::Relaxed);
        if v != vol_now {
            vol_now = v;
            if let Some(e) = volume_el.lock().unwrap().as_ref() {
                e.set_property("volume", v as f64 / 100.0);
            }
        }
        // Stats.
        let (ptx, prx) = mpsc::channel::<Option<gst::Structure>>();
        let p = gst::Promise::with_change_func(move |r| {
            let _ = ptx.send(r.ok().flatten().map(|s| s.to_owned()));
        });
        webrtc.emit_by_name::<()>("get-stats", &[&None::<gst::Pad>, &p]);
        let mut st = Stats { height: height.load(Ordering::Relaxed), ..Stats::default() };
        if let Ok(Some(s)) = prx.recv_timeout(Duration::from_millis(500)) {
            let (bytes, rtt, lost, got) = read_stats(&s);
            let dt = last_t.elapsed().as_secs_f32().max(0.1);
            st.mbps = (bytes.saturating_sub(last_bytes)) as f32 * 8.0 / dt / 1e6;
            st.delay_ms = rtt;
            st.loss = if got + lost > 0 { lost as f32 / (got + lost) as f32 } else { 0.0 };
            st.session_bytes = bytes;
            last_bytes = bytes;
        }
        let frames = decoded.load(Ordering::Relaxed);
        st.fps = ((frames - last_frames) as f32 / last_t.elapsed().as_secs_f32().max(0.1)).round() as u32;
        last_frames = frames;
        last_t = Instant::now();
        tx.send(Event::Stats(st.clone()));
        if ka.elapsed() > Duration::from_secs(60) {
            ka = Instant::now();
            let _ = ses.keepalive();
        }
        if minute.elapsed() > Duration::from_secs(60) {
            minute = Instant::now();
            tx.send(Event::Data(st.session_bytes.saturating_sub(counted)));
            counted = st.session_bytes;
        }
    }
    tx.send(Event::Data(last_bytes.saturating_sub(counted)));
    let _ = pipeline.set_state(gst::State::Null);
    Ok(())
}

/// From webrtcbin's stats: bytes received and sent, the round trip (ms), packets lost and received.
#[cfg(feature = "stream")]
fn read_stats(s: &gstreamer::StructureRef) -> (u64, Option<u32>, u64, u64) {
    let mut bytes = 0u64;
    let mut rtt: Option<f64> = None;
    let mut lost = 0u64;
    let mut got = 0u64;
    for (_, v) in s.iter() {
        let Ok(inner) = v.get::<gstreamer::Structure>() else { continue };
        let name = inner.name().to_string();
        let u = |k: &str| inner.get::<u64>(k).ok().or_else(|| inner.get::<u32>(k).ok().map(u64::from)).or_else(|| inner.get::<i64>(k).ok().map(|x| x.max(0) as u64)).unwrap_or(0);
        if name.contains("inbound-rtp") {
            bytes += u("bytes-received");
            got += u("packets-received");
            lost += inner.get::<i64>("packets-lost").map(|x| x.max(0) as u64).unwrap_or(0);
        } else if name.contains("outbound-rtp") && !name.contains("remote") {
            bytes += u("bytes-sent");
        }
        for k in ["round-trip-time", "current-round-trip-time"] {
            if let Ok(x) = inner.get::<f64>(k) {
                if x > 0.0 {
                    rtt = Some(x);
                }
            }
        }
    }
    (bytes, rtt.map(|x| (x * 1000.0).round() as u32), lost, got)
}

/// The stand-in stream for CI's pictures (HEARTH_CRP_FIXTURE): the steps in turn, then "started" with made-up stats
/// (the window shows the fixture's still picture), so every streaming page can be pictured. Never used outside CI.
/// How many frames the stand-in has made (CI's check that frames keep coming).
pub static STAND_IN_FRAMES: AtomicU32 = AtomicU32::new(0);

pub fn start_stand_in(out: (u32, u32), step_only: Option<&'static str>) {
    stop();
    let tx = Tx::new();
    let flags = Arc::new(spad::Flags::default());
    let ctl = Arc::new(Control { stop: AtomicBool::new(false), volume: AtomicU32::new(80), flags });
    *RUNNING.lock().unwrap() = Some(ctl.clone());
    let _ = std::thread::Builder::new().name("crp-stand-in".into()).spawn(move || {
        let send = |e: Event| {
            tx.send(e);
        };
        send(Event::Step(Step::Waking));
        send(Event::Step(Step::Woke(12)));
        send(Event::Step(Step::Connecting));
        if step_only == Some("connecting") {
            while !ctl.stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(200));
            }
            return;
        }
        if step_only == Some("trouble") {
            send(Event::Failed { why: "Living room woke up, but the picture couldn't get through to this laptop in time.".into(), woke: true });
            send(Event::Ended);
            *RUNNING.lock().unwrap() = None;
            return;
        }
        send(Event::Step(Step::Picture));
        send(Event::Started);
        FRAME_PENDING.store(true, Ordering::Release);
        send(Event::Frame);
        let hd = out.1 >= 1080;
        let st = if hd {
            Stats { delay_ms: Some(38), mbps: 11.8, height: 1080, fps: 60, loss: 0.0, session_bytes: 1_600_000_000 }
        } else {
            Stats { delay_ms: Some(34), mbps: 7.9, height: 720, fps: 60, loss: 0.002, session_bytes: 400_000_000 }
        };
        send(Event::Stats(st));
        // A moving stand-in: the fixture's picture as 30 new frames a second, each a little different (a corner
        // block's shade), so the window is tested as it is with a real stream (frames replacing frames).
        let base = crate::fixture().and_then(|f| image::open(f.join("stream-frame.jpg")).ok()).map(|i| i.resize_exact(out.0, out.1, image::imageops::FilterType::Triangle).to_rgba8());
        let mut n: u32 = 0;
        while !ctl.stop.load(Ordering::Relaxed) {
            if let Some(b) = &base {
                let mut px = b.as_raw().clone();
                let shade = (n % 64) as u8 * 2;
                for y in 0..6usize {
                    for x in 0..6usize {
                        let i = (y * out.0 as usize + x) * 4;
                        px[i] = shade;
                        px[i + 1] = shade;
                        px[i + 2] = shade;
                    }
                }
                *FRAME.lock().unwrap() = Some(Frame { w: out.0, h: out.1, rgba: px });
                STAND_IN_FRAMES.fetch_add(1, Ordering::Relaxed);
                if !FRAME_PENDING.swap(true, Ordering::AcqRel) {
                    send(Event::Frame);
                }
                n += 1;
                std::thread::sleep(Duration::from_millis(33));
            } else {
                std::thread::sleep(Duration::from_millis(200));
            }
        }
        send(Event::Ended);
        *RUNNING.lock().unwrap() = None;
    });
}
