//! Going live to Twitch and YouTube at once (board 12, Where): OBS streams to one place at a time, so with two or
//! more places Hearth Studio runs its own relay: OBS streams to it on this PC (rtmp://127.0.0.1:19350/hearth, set as
//! OBS's custom server while you're live), and the relay, inside Hearth Studio itself, copies that one stream,
//! untouched (no second encode), to every place you picked over RTMPS (TLS); a place that drops out doesn't stop the
//! others. When you end the stream the relay stops and OBS is set back to your first place, so OBS on its own still
//! goes there.
//! Stream keys: you type each into Hearth Studio once; it keeps them in your keyring (secret-tool, the key on its
//! stdin), never in a file or a log. While you're live they are only in Hearth Studio's memory: the relay is threads
//! of this app, not another program, so no key is ever on a command line or in an environment that other accounts on
//! the PC could read (/proc/<pid>/cmdline, ps). Errors never print a key either.

use bytes::Bytes;
use rml_rtmp::handshake::{Handshake, HandshakeProcessResult, PeerType};
use rml_rtmp::sessions::{
    ClientSession, ClientSessionConfig, ClientSessionEvent, ClientSessionResult, PublishRequestType, ServerSession, ServerSessionConfig,
    ServerSessionEvent, ServerSessionResult, StreamMetadata,
};
use rml_rtmp::time::RtmpTimestamp;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub const LISTEN: &str = "rtmp://127.0.0.1:19350/hearth";
pub const KEY: &str = "live";
const ADDR: &str = "127.0.0.1:19350";
pub const PLACES: [&str; 3] = ["Twitch", "YouTube", "Kick"];
/// How much of the stream may wait for a place that's slow (about 10 seconds); past it that place skips ahead to the
/// next whole picture instead of holding the others up.
const QUEUE: usize = 1500;

struct Relay {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

static RELAY: Mutex<Option<Relay>> = Mutex::new(None);

/// Where a place takes a stream. The key is never part of it (it goes in RTMP's publish message, inside TLS).
#[derive(Clone, Debug, PartialEq)]
pub struct Ingest {
    pub tls: bool,
    pub host: String,
    pub port: u16,
    pub app: String,
}

impl Ingest {
    pub fn tc_url(&self) -> String {
        format!("{}://{}:{}/{}", if self.tls { "rtmps" } else { "rtmp" }, self.host, self.port, self.app)
    }
}

/// Each place's RTMPS ingest.
pub fn ingest(place: &str) -> Ingest {
    let (host, app) = match place {
        "YouTube" => ("a.rtmps.youtube.com", "live2"),
        "Kick" => ("fa723fc1b171.global-contribute.live-video.net", "app"),
        _ => ("ingest.global-contribute.live-video.net", "app"),
    };
    Ingest { tls: true, host: host.into(), port: 443, app: app.into() }
}

/// OBS's own name for a place's service (rtmp_common).
pub fn service(place: &str) -> (&'static str, &'static str) {
    match place {
        "YouTube" => ("YouTube - RTMPS", "rtmps://a.rtmps.youtube.com:443/live2"),
        "Kick" => ("Kick", "auto"),
        _ => ("Twitch", "auto"),
    }
}

/// One place the relay sends to. No Debug on purpose: nothing may print the key.
#[derive(Clone)]
pub struct Dest {
    pub at: Ingest,
    key: String,
}

#[derive(Clone)]
enum Media {
    Meta(StreamMetadata),
    Video(Bytes, RtmpTimestamp),
    Audio(Bytes, RtmpTimestamp),
}

pub fn running() -> bool {
    RELAY.lock().ok().map(|g| g.as_ref().map(|r| !r.thread.is_finished()).unwrap_or(false)).unwrap_or(false)
}

/// The relay started (before OBS starts streaming to it). `targets`: (place, key).
pub fn start(targets: &[(String, String)]) -> Result<(), String> {
    start_to(targets.iter().map(|(p, k)| Dest { at: ingest(p), key: k.clone() }).collect(), ADDR)
}

fn start_to(dests: Vec<Dest>, addr: &str) -> Result<(), String> {
    stop();
    let l = TcpListener::bind(addr).map_err(|e| format!("the relay couldn't listen on this PC ({e})"))?;
    l.set_nonblocking(true).map_err(|e| format!("the relay couldn't listen on this PC ({e})"))?;
    let stop = Arc::new(AtomicBool::new(false));
    let s2 = stop.clone();
    let thread = std::thread::Builder::new()
        .name("hearth-studio-relay".into())
        .spawn(move || listen(l, dests, s2))
        .map_err(|e| format!("the relay couldn't start ({e})"))?;
    if let Ok(mut g) = RELAY.lock() {
        *g = Some(Relay { stop, thread });
    }
    Ok(())
}

/// The relay stopped: OBS's side closed, each place told the stream ended and closed.
pub fn stop() {
    let r = RELAY.lock().ok().and_then(|mut g| g.take());
    if let Some(r) = r {
        r.stop.store(true, Ordering::SeqCst);
        let _ = r.thread.join();
    }
}

fn waiting(e: &std::io::Error) -> bool {
    matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted)
}

fn listen(l: TcpListener, dests: Vec<Dest>, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) {
        match l.accept() {
            Ok((s, _)) => {
                let _ = s.set_nonblocking(false);
                publisher(s, &dests, &stop);
            }
            Err(_) => std::thread::sleep(Duration::from_millis(100)),
        }
    }
}

/// RTMP's handshake, either side. Returns what came after it.
fn shake<S: Read + Write>(s: &mut S, peer: PeerType, stop: &AtomicBool) -> Result<Vec<u8>, String> {
    let client = peer == PeerType::Client;
    let mut hs = Handshake::new(peer);
    if client {
        let p = hs.generate_outbound_p0_and_p1().map_err(|_| "handshake".to_string())?;
        s.write_all(&p).map_err(|e| e.to_string())?;
    }
    let mut buf = vec![0u8; 16384];
    let until = Instant::now() + Duration::from_secs(15);
    loop {
        if stop.load(Ordering::SeqCst) || Instant::now() > until {
            return Err("no answer".into());
        }
        let n = match s.read(&mut buf) {
            Ok(0) => return Err("closed".into()),
            Ok(n) => n,
            Err(e) if waiting(&e) => continue,
            Err(e) => return Err(e.to_string()),
        };
        match hs.process_bytes(&buf[..n]).map_err(|_| "handshake".to_string())? {
            HandshakeProcessResult::InProgress { response_bytes } => s.write_all(&response_bytes).map_err(|e| e.to_string())?,
            HandshakeProcessResult::Completed { response_bytes, remaining_bytes } => {
                s.write_all(&response_bytes).map_err(|e| e.to_string())?;
                return Ok(remaining_bytes);
            }
        }
    }
}

/// A place being sent to: its queue, and whether it fell behind (then it waits for the next whole picture).
struct Out {
    tx: SyncSender<Media>,
    behind: bool,
}

fn is_keyframe(d: &[u8]) -> bool {
    !d.is_empty() && (d[0] >> 4) & 0x07 == 1
}

fn is_video_head(d: &[u8]) -> bool {
    if d.is_empty() {
        return false;
    }
    if d[0] & 0x80 != 0 {
        d[0] & 0x0f == 0 // enhanced RTMP (HEVC, AV1): SequenceStart
    } else {
        d.len() >= 2 && d[0] & 0x0f == 7 && d[1] == 0 // H.264: its sequence header
    }
}

fn is_audio_head(d: &[u8]) -> bool {
    d.len() >= 2 && d[0] >> 4 == 10 && d[1] == 0 // AAC: its sequence header
}

/// What a place needs before the picture can start: the stream's details and its codec headers.
#[derive(Default)]
struct Heads {
    meta: Option<StreamMetadata>,
    video: Option<(Bytes, RtmpTimestamp)>,
    audio: Option<(Bytes, RtmpTimestamp)>,
}

fn fan_out(outs: &mut [Out], heads: &Heads, m: Media) {
    let key = matches!(&m, Media::Video(d, _) if is_keyframe(d) && !is_video_head(d));
    for o in outs.iter_mut() {
        if o.behind {
            if !key {
                continue;
            }
            // caught up: the headers again, then this whole picture
            let mut again: Vec<Media> = Vec::new();
            if let Some(md) = &heads.meta {
                again.push(Media::Meta(md.clone()));
            }
            if let Some((d, t)) = &heads.video {
                again.push(Media::Video(d.clone(), *t));
            }
            if let Some((d, t)) = &heads.audio {
                again.push(Media::Audio(d.clone(), *t));
            }
            if again.into_iter().any(|x| o.tx.try_send(x).is_err()) {
                continue;
            }
            o.behind = false;
        }
        match o.tx.try_send(m.clone()) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => o.behind = true,
            Err(TrySendError::Disconnected(_)) => {}
        }
    }
}

/// OBS streaming to the relay: each place gets its own connection for as long as OBS stays.
fn publisher(mut s: TcpStream, dests: &[Dest], stop: &Arc<AtomicBool>) {
    let _ = s.set_read_timeout(Some(Duration::from_millis(200)));
    let _ = s.set_write_timeout(Some(Duration::from_secs(5)));
    let _ = s.set_nodelay(true);
    let rest = match shake(&mut s, PeerType::Server, stop) {
        Ok(r) => r,
        Err(_) => return,
    };
    let (mut sess, mut pending) = match ServerSession::new(ServerSessionConfig::new()) {
        Ok(x) => x,
        Err(_) => return,
    };
    let mut outs: Vec<Out> = Vec::new();
    for d in dests {
        let (tx, rx) = sync_channel::<Media>(QUEUE);
        let d = d.clone();
        let st = stop.clone();
        if std::thread::Builder::new().name("hearth-studio-push".into()).spawn(move || push(d, rx, st)).is_ok() {
            outs.push(Out { tx, behind: false });
        }
    }
    let mut heads = Heads::default();
    if !rest.is_empty() {
        match sess.handle_input(&rest) {
            Ok(r) => pending.extend(r),
            Err(_) => return,
        }
    }
    let mut buf = vec![0u8; 65536];
    loop {
        while !pending.is_empty() {
            let now: Vec<ServerSessionResult> = pending.drain(..).collect();
            for r in now {
                match r {
                    ServerSessionResult::OutboundResponse(p) => {
                        if s.write_all(&p.bytes).is_err() {
                            return;
                        }
                    }
                    ServerSessionResult::RaisedEvent(ev) => match ev {
                        ServerSessionEvent::ConnectionRequested { request_id, .. }
                        | ServerSessionEvent::ReleaseStreamRequested { request_id, .. }
                        | ServerSessionEvent::PublishStreamRequested { request_id, .. } => match sess.accept_request(request_id) {
                            Ok(r) => pending.extend(r),
                            Err(_) => return,
                        },
                        ServerSessionEvent::StreamMetadataChanged { metadata, .. } => {
                            heads.meta = Some(metadata.clone());
                            fan_out(&mut outs, &heads, Media::Meta(metadata));
                        }
                        ServerSessionEvent::VideoDataReceived { data, timestamp, .. } => {
                            if is_video_head(&data) {
                                heads.video = Some((data.clone(), timestamp));
                            }
                            fan_out(&mut outs, &heads, Media::Video(data, timestamp));
                        }
                        ServerSessionEvent::AudioDataReceived { data, timestamp, .. } => {
                            if is_audio_head(&data) {
                                heads.audio = Some((data.clone(), timestamp));
                            }
                            fan_out(&mut outs, &heads, Media::Audio(data, timestamp));
                        }
                        ServerSessionEvent::PublishStreamFinished { .. } => return,
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        if stop.load(Ordering::SeqCst) {
            return;
        }
        let n = match s.read(&mut buf) {
            Ok(0) => return,
            Ok(n) => n,
            Err(e) if waiting(&e) => continue,
            Err(_) => return,
        };
        match sess.handle_input(&buf[..n]) {
            Ok(r) => pending.extend(r),
            Err(_) => return,
        }
    }
    // returning drops `outs`: each place's queue closes, it says the stream ended and closes
}

/// A connection to a place, plain or TLS.
enum Conn {
    Plain(TcpStream),
    Tls(Box<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>),
}

impl Conn {
    fn tcp(&self) -> &TcpStream {
        match self {
            Conn::Plain(t) => t,
            Conn::Tls(t) => &t.sock,
        }
    }
}

impl Read for Conn {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Conn::Plain(t) => t.read(b),
            Conn::Tls(t) => t.read(b),
        }
    }
}

impl Write for Conn {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        match self {
            Conn::Plain(t) => t.write(b),
            Conn::Tls(t) => t.write(b),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Conn::Plain(t) => t.flush(),
            Conn::Tls(t) => t.flush(),
        }
    }
}

/// TLS to a place, checked against the usual web certificate authorities.
fn tls(host: &str, tcp: TcpStream) -> Result<rustls::StreamOwned<rustls::ClientConnection, TcpStream>, String> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let cfg = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_root_certificates(roots)
        .with_no_client_auth();
    let name = rustls::pki_types::ServerName::try_from(host.to_string()).map_err(|e| e.to_string())?;
    let conn = rustls::ClientConnection::new(Arc::new(cfg), name).map_err(|e| e.to_string())?;
    Ok(rustls::StreamOwned::new(conn, tcp))
}

/// Sends what the session wants sent; hands back what it raised.
fn out(c: &mut Conn, rs: Vec<ClientSessionResult>) -> Result<Vec<ClientSessionEvent>, String> {
    let mut evs = Vec::new();
    for r in rs {
        match r {
            ClientSessionResult::OutboundResponse(p) => c.write_all(&p.bytes).map_err(|e| e.to_string())?,
            ClientSessionResult::RaisedEvent(e) => evs.push(e),
            _ => {}
        }
    }
    Ok(evs)
}

fn push(d: Dest, rx: Receiver<Media>, stop: Arc<AtomicBool>) {
    // a place that fails just drops out (the others go on); nothing is logged
    let _ = push_to(&d, &rx, &stop);
}

fn push_to(d: &Dest, rx: &Receiver<Media>, stop: &AtomicBool) -> Result<(), String> {
    let addr = (d.at.host.as_str(), d.at.port).to_socket_addrs().map_err(|e| e.to_string())?.next().ok_or_else(|| "no address".to_string())?;
    let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(10)).map_err(|e| e.to_string())?;
    let _ = tcp.set_nodelay(true);
    let _ = tcp.set_write_timeout(Some(Duration::from_secs(10)));
    let _ = tcp.set_read_timeout(Some(Duration::from_millis(300)));
    let mut c = if d.at.tls { Conn::Tls(Box::new(tls(&d.at.host, tcp)?)) } else { Conn::Plain(tcp) };
    let rest = shake(&mut c, PeerType::Client, stop)?;
    let mut cfg = ClientSessionConfig::new();
    cfg.tc_url = Some(d.at.tc_url());
    let (mut sess, first) = ClientSession::new(cfg).map_err(|_| "session".to_string())?;
    let mut evs = out(&mut c, first)?;
    let r = sess.request_connection(d.at.app.clone()).map_err(|_| "connect".to_string())?;
    evs.extend(out(&mut c, vec![r])?);
    if !rest.is_empty() {
        let r = sess.handle_input(&rest).map_err(|_| "rtmp".to_string())?;
        evs.extend(out(&mut c, r)?);
    }
    let mut buf = vec![0u8; 65536];
    let until = Instant::now() + Duration::from_secs(20);
    let mut live = false;
    while !live {
        for e in evs.drain(..).collect::<Vec<_>>() {
            match e {
                ClientSessionEvent::ConnectionRequestAccepted => {
                    let r = sess.request_publishing(d.key.clone(), PublishRequestType::Live).map_err(|_| "publish".to_string())?;
                    out(&mut c, vec![r])?;
                }
                ClientSessionEvent::PublishRequestAccepted => live = true,
                ClientSessionEvent::ConnectionRequestRejected { .. } => return Err("refused".into()),
                _ => {}
            }
        }
        if live {
            break;
        }
        if stop.load(Ordering::SeqCst) || Instant::now() > until {
            return Err("no answer".into());
        }
        let n = match c.read(&mut buf) {
            Ok(0) => return Err("closed".into()),
            Ok(n) => n,
            Err(e) if waiting(&e) => continue,
            Err(e) => return Err(e.to_string()),
        };
        let r = sess.handle_input(&buf[..n]).map_err(|_| "rtmp".to_string())?;
        evs.extend(out(&mut c, r)?);
    }
    // live: the stream as it comes; now and then what the place says (its acknowledgements and pings)
    let _ = c.tcp().set_read_timeout(Some(Duration::from_millis(1)));
    let mut heard = Instant::now();
    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(m) => {
                let r = match m {
                    Media::Meta(md) => sess.publish_metadata(&md),
                    Media::Video(b, t) => sess.publish_video_data(b, t, false),
                    Media::Audio(b, t) => sess.publish_audio_data(b, t, false),
                }
                .map_err(|_| "rtmp".to_string())?;
                out(&mut c, vec![r])?;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if heard.elapsed() > Duration::from_millis(250) {
            heard = Instant::now();
            match c.read(&mut buf) {
                Ok(0) => return Err("closed".into()),
                Ok(n) => {
                    let r = sess.handle_input(&buf[..n]).map_err(|_| "rtmp".to_string())?;
                    out(&mut c, r)?;
                }
                Err(e) if waiting(&e) => {}
                Err(e) => return Err(e.to_string()),
            }
        }
    }
    // the stream ended: tell the place, then close
    if let Ok(r) = sess.stop_publishing() {
        let _ = out(&mut c, r);
    }
    if let Conn::Tls(t) = &mut c {
        t.conn.send_close_notify();
    }
    let _ = c.flush();
    let _ = c.tcp().shutdown(std::net::Shutdown::Both);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    // A pretend key; the real ones only ever come from the keyring.
    const FAKE: &str = "live_000000000_FAKEKEYhearthstudiotest";

    #[test]
    fn places_and_obs_services() {
        assert_eq!(ingest("YouTube").tc_url(), "rtmps://a.rtmps.youtube.com:443/live2");
        assert_eq!(ingest("Twitch").tc_url(), "rtmps://ingest.global-contribute.live-video.net:443/app");
        assert!(PLACES.iter().all(|p| ingest(p).tls), "every place is reached over RTMPS (TLS)");
        assert_eq!(service("YouTube").0, "YouTube - RTMPS");
    }

    #[test]
    fn tls_is_ready() {
        // the TLS side builds (ring, the web's certificate authorities) without going anywhere
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let t = TcpStream::connect(l.local_addr().unwrap()).unwrap();
        assert!(tls("a.rtmps.youtube.com", t).is_ok());
    }

    #[test]
    fn the_relay_starts_no_program() {
        // no other program is started for the relay, so no key can be on a command line or in an environment
        let src = include_str!("relay.rs");
        let code = src.split("#[cfg(test)]").next().unwrap();
        for bad in [concat!("Command", "::"), concat!("process", "::"), concat!("set", "_var"), "ffmpeg"] {
            assert!(!code.contains(bad), "relay.rs uses {bad}");
        }
    }

    fn proc_has(needle: &[u8]) -> Vec<String> {
        let mut hits = Vec::new();
        for e in std::fs::read_dir("/proc").unwrap().flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if !n.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            for f in ["cmdline", "environ"] {
                if let Ok(b) = std::fs::read(e.path().join(f)) {
                    if b.windows(needle.len()).any(|w| w == needle) {
                        hits.push(format!("/proc/{n}/{f}"));
                    }
                }
            }
        }
        hits
    }

    /// A pretend Twitch: takes one stream, says which app and key it was given and how many pictures came.
    fn pretend_place(l: TcpListener, said: std::sync::mpsc::Sender<(String, String, usize)>) {
        let (mut s, _) = l.accept().unwrap();
        s.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
        let no = AtomicBool::new(false);
        let rest = shake(&mut s, PeerType::Server, &no).unwrap();
        let (mut sess, mut pending) = ServerSession::new(ServerSessionConfig::new()).unwrap();
        pending.extend(sess.handle_input(&rest).unwrap());
        let (mut app, mut key, mut pictures) = (String::new(), String::new(), 0usize);
        let mut buf = vec![0u8; 65536];
        let until = Instant::now() + Duration::from_secs(20);
        while Instant::now() < until {
            for r in pending.drain(..).collect::<Vec<_>>() {
                match r {
                    ServerSessionResult::OutboundResponse(p) => s.write_all(&p.bytes).unwrap(),
                    ServerSessionResult::RaisedEvent(ServerSessionEvent::ConnectionRequested { request_id, app_name }) => {
                        app = app_name;
                        pending.extend(sess.accept_request(request_id).unwrap());
                    }
                    ServerSessionResult::RaisedEvent(ServerSessionEvent::PublishStreamRequested { request_id, stream_key, .. }) => {
                        key = stream_key;
                        pending.extend(sess.accept_request(request_id).unwrap());
                    }
                    ServerSessionResult::RaisedEvent(ServerSessionEvent::VideoDataReceived { .. }) => pictures += 1,
                    ServerSessionResult::RaisedEvent(ServerSessionEvent::PublishStreamFinished { .. }) => {
                        let _ = said.send((app, key, pictures));
                        return;
                    }
                    _ => {}
                }
            }
            match s.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => pending.extend(sess.handle_input(&buf[..n]).unwrap()),
                Err(e) if waiting(&e) => {}
                Err(_) => break,
            }
        }
        let _ = said.send((app, key, pictures));
    }

    #[test]
    fn one_stream_to_every_place_and_no_key_on_any_command_line() {
        let place = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = place.local_addr().unwrap().port();
        let (said_tx, said) = channel();
        let pt = std::thread::spawn(move || pretend_place(place, said_tx));
        let at = Ingest { tls: false, host: "127.0.0.1".into(), port, app: "app".into() };
        start_to(vec![Dest { at, key: FAKE.into() }], "127.0.0.1:19351").unwrap();
        assert!(running());

        // a pretend OBS streams to the relay
        let mut obs = TcpStream::connect("127.0.0.1:19351").unwrap();
        obs.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
        let no = AtomicBool::new(false);
        let rest = shake(&mut obs, PeerType::Client, &no).unwrap();
        let mut cfg = ClientSessionConfig::new();
        cfg.tc_url = Some(LISTEN.into());
        let (mut sess, first) = ClientSession::new(cfg).unwrap();
        let mut pending = first;
        pending.push(sess.request_connection("hearth".into()).unwrap());
        pending.extend(sess.handle_input(&rest).unwrap());
        let mut buf = vec![0u8; 65536];
        let mut live = false;
        let until = Instant::now() + Duration::from_secs(20);
        while !live && Instant::now() < until {
            for r in pending.drain(..).collect::<Vec<_>>() {
                match r {
                    ClientSessionResult::OutboundResponse(p) => obs.write_all(&p.bytes).unwrap(),
                    ClientSessionResult::RaisedEvent(ClientSessionEvent::ConnectionRequestAccepted) => {
                        pending.push(sess.request_publishing(KEY.into(), PublishRequestType::Live).unwrap())
                    }
                    ClientSessionResult::RaisedEvent(ClientSessionEvent::PublishRequestAccepted) => live = true,
                    _ => {}
                }
            }
            if live || !pending.is_empty() {
                continue;
            }
            match obs.read(&mut buf) {
                Ok(n) if n > 0 => pending.extend(sess.handle_input(&buf[..n]).unwrap()),
                _ => {}
            }
        }
        assert!(live, "the relay took OBS's stream");
        let mut md = StreamMetadata::new();
        md.video_width = Some(1920);
        md.video_height = Some(1080);
        let mut send = |r: ClientSessionResult| {
            if let ClientSessionResult::OutboundResponse(p) = r {
                obs.write_all(&p.bytes).unwrap();
            }
        };
        send(sess.publish_metadata(&md).unwrap());
        send(sess.publish_video_data(Bytes::from_static(&[0x17, 0, 0, 0, 0, 1, 2, 3]), RtmpTimestamp::new(0), false).unwrap());
        send(sess.publish_audio_data(Bytes::from_static(&[0xaf, 0, 0x12, 0x10]), RtmpTimestamp::new(0), false).unwrap());
        for i in 0..30u32 {
            let d: &'static [u8] = if i % 10 == 0 { &[0x17, 1, 0, 0, 0, 9, 9] } else { &[0x27, 1, 0, 0, 0, 8, 8] };
            send(sess.publish_video_data(Bytes::from_static(d), RtmpTimestamp::new(i * 33), false).unwrap());
            send(sess.publish_audio_data(Bytes::from_static(&[0xaf, 1, 7, 7]), RtmpTimestamp::new(i * 33), false).unwrap());
        }
        std::thread::sleep(Duration::from_millis(1500));

        // live: the key is in no process's command line or environment on this PC
        let hits = proc_has(FAKE.as_bytes());
        assert!(hits.is_empty(), "a stream key is readable at {hits:?}");

        // OBS ends the stream; the relay stops and tells the place
        for r in sess.stop_publishing().unwrap() {
            send(r);
        }
        drop(obs);
        stop();
        assert!(!running());
        let (app, key, pictures) = said.recv_timeout(Duration::from_secs(20)).unwrap();
        pt.join().unwrap();
        assert_eq!(app, "app");
        assert!(key == FAKE, "the place got the key it was given");
        assert!(pictures >= 31, "the place got the pictures ({pictures})");
        // and it can start again on the same port
        start_to(Vec::new(), "127.0.0.1:19351").unwrap();
        stop();
    }
}
