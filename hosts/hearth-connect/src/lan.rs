//! KDE Connect over the local network (its "LAN backend", protocol version 8):
//! - Discovery: each device broadcasts its identity over UDP to port 1716. The one that hears it opens a TCP
//!   connection to the port in it (1716-1764) and sends its own identity as the first line.
//! - TLS on that connection with the roles reversed: whoever opened the TCP connection is the TLS server, and both
//!   sides show their certificate (self-signed; the common name is the device id). After TLS, both send their
//!   identity again (version 8), now protected.
//! - Then packets, one JSON line each way. A file goes over a second connection: the sender listens on 1739-1764
//!   and says the port in the packet; the receiver connects, the sender is the TLS server again.
//! A paired device must show the certificate it paired with; any other is refused.

use crate::daemon::Daemon;
use crate::identity::Identity;
use crate::packet::{self, Packet};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{DigitallySignedStruct, DistinguishedName, SignatureScheme};
use socket2::{Domain, Protocol, SockRef, Socket, TcpKeepalive, Type};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::watch;

pub trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}

/// Longest line accepted before a link is up (an identity) and after (a packet; KDE Connect's own limits).
pub const MAX_IDENTITY: usize = 64 * 1024;
pub const MAX_PACKET: usize = 32 * 1024 * 1024;

fn provider() -> Arc<CryptoProvider> { Arc::new(rustls::crypto::ring::default_provider()) }
fn key(me: &Identity) -> PrivateKeyDer<'static> { PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(me.key_der.clone())) }

/// Certificates are checked after the handshake (against the paired one), not against any authority.
#[derive(Debug)]
struct AnyCert(Arc<CryptoProvider>);

impl ClientCertVerifier for AnyCert {
    fn offer_client_auth(&self) -> bool { true }
    fn client_auth_mandatory(&self) -> bool { true }
    fn root_hint_subjects(&self) -> &[DistinguishedName] { &[] }
    fn verify_client_cert(&self, _: &CertificateDer<'_>, _: &[CertificateDer<'_>], _: UnixTime) -> Result<ClientCertVerified, rustls::Error> { Ok(ClientCertVerified::assertion()) }
    fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(m, c, d, &self.0.signature_verification_algorithms)
    }
    fn verify_tls13_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(m, c, d, &self.0.signature_verification_algorithms)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> { self.0.signature_verification_algorithms.supported_schemes() }
}

impl ServerCertVerifier for AnyCert {
    fn verify_server_cert(&self, _: &CertificateDer<'_>, _: &[CertificateDer<'_>], _: &ServerName<'_>, _: &[u8], _: UnixTime) -> Result<ServerCertVerified, rustls::Error> { Ok(ServerCertVerified::assertion()) }
    fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(m, c, d, &self.0.signature_verification_algorithms)
    }
    fn verify_tls13_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(m, c, d, &self.0.signature_verification_algorithms)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> { self.0.signature_verification_algorithms.supported_schemes() }
}

pub fn acceptor(me: &Identity) -> Result<tokio_rustls::TlsAcceptor, String> {
    let p = provider();
    let cfg = rustls::ServerConfig::builder_with_provider(p.clone()).with_safe_default_protocol_versions().map_err(|e| e.to_string())?
        .with_client_cert_verifier(Arc::new(AnyCert(p)))
        .with_single_cert(vec![CertificateDer::from(me.cert_der.clone())], key(me)).map_err(|e| e.to_string())?;
    Ok(tokio_rustls::TlsAcceptor::from(Arc::new(cfg)))
}

pub fn connector(me: &Identity) -> Result<tokio_rustls::TlsConnector, String> {
    let p = provider();
    let cfg = rustls::ClientConfig::builder_with_provider(p.clone()).with_safe_default_protocol_versions().map_err(|e| e.to_string())?
        .dangerous().with_custom_certificate_verifier(Arc::new(AnyCert(p)))
        .with_client_auth_cert(vec![CertificateDer::from(me.cert_der.clone())], key(me)).map_err(|e| e.to_string())?;
    Ok(tokio_rustls::TlsConnector::from(Arc::new(cfg)))
}

fn server_name(device_id: &str) -> ServerName<'static> {
    ServerName::try_from(device_id.to_string()).unwrap_or_else(|_| ServerName::try_from("kdeconnect".to_string()).expect("static name"))
}

/// Dead links show up in about a minute (a phone that left the Wi-Fi), as KDE Connect does.
fn keepalive(s: &TcpStream) {
    let r = SockRef::from(s);
    let ka = TcpKeepalive::new().with_time(Duration::from_secs(30)).with_interval(Duration::from_secs(10));
    #[cfg(any(target_os = "linux", target_os = "android"))]
    let ka = ka.with_retries(3);
    let _ = r.set_tcp_keepalive(&ka);
    let _ = s.set_nodelay(true);
}

/// One line, read a byte at a time (nothing past the newline may be taken: TLS starts right after it).
async fn read_line_raw(s: &mut TcpStream, max: usize) -> Option<String> {
    let mut buf = Vec::with_capacity(512);
    let mut b = [0u8; 1];
    loop {
        match s.read(&mut b).await { Ok(1) => {}, _ => return None }
        if b[0] == b'\n' { break; }
        buf.push(b[0]);
        if buf.len() > max { return None; }
    }
    String::from_utf8(buf).ok()
}

/// What the other side said in its identity packet.
#[derive(Clone, Debug)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub protocol: i64,
    pub incoming: Vec<String>,
    pub outgoing: Vec<String>,
    pub cert: Vec<u8>,
    pub ip: IpAddr,
}

fn peer_from(p: &Packet, cert: Vec<u8>, ip: IpAddr) -> Option<Peer> {
    if p.kind != "kdeconnect.identity" { return None; }
    let id = p.str("deviceId")?.to_string();
    if !crate::identity::valid_device_id(&id) { return None; }
    let list = |k: &str| p.body.get(k).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    let name = crate::identity::filter_name(p.str("deviceName").unwrap_or(""));
    Some(Peer { name: if name.is_empty() { "Phone".into() } else { name }, kind: p.str("deviceType").unwrap_or("phone").to_string(),
        protocol: p.int("protocolVersion").unwrap_or(0), incoming: list("incomingCapabilities"), outgoing: list("outgoingCapabilities"), cert, id, ip })
}

/// Both sides' identities over the fresh TLS link; then the link belongs to the daemon.
async fn secure(d: Arc<Daemon>, tls: Box<dyn Io>, cert: Option<Vec<u8>>, expect_id: String, ip: IpAddr) {
    let Some(cert) = cert else { return };
    let (rd, mut wr) = tokio::io::split(tls);
    let mut rd = BufReader::new(rd);
    if wr.write_all(d.identity_packet(None, None).line().as_bytes()).await.is_err() { return; }
    let _ = wr.flush().await;
    let mut line = String::new();
    let got = tokio::time::timeout(Duration::from_secs(5), read_limited(&mut rd, &mut line, MAX_IDENTITY)).await;
    if !matches!(got, Ok(Ok(n)) if n > 0) { return; }
    let Some(p) = Packet::parse(&line) else { return };
    let Some(peer) = peer_from(&p, cert, ip) else { return };
    if peer.id != expect_id || peer.protocol < packet::PROTOCOL_VERSION { return; }
    // The certificate must be that device's: its name is the device id.
    match crate::identity::cert_info(&peer.cert) { Some((_, cn)) if cn == peer.id => {}, _ => return }
    d.attach(peer, rd, wr).await;
}

/// Reads one line of at most `max` bytes (longer: an error).
pub async fn read_limited<R: tokio::io::AsyncBufRead + Unpin>(r: &mut R, out: &mut String, max: usize) -> std::io::Result<usize> {
    out.clear();
    let mut buf = Vec::new();
    loop {
        let avail = r.fill_buf().await?;
        if avail.is_empty() { break; }
        let (take, done) = match avail.iter().position(|b| *b == b'\n') { Some(i) => (i + 1, true), None => (avail.len(), false) };
        buf.extend_from_slice(&avail[..take]);
        r.consume(take);
        if buf.len() > max { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "line too long")); }
        if done { break; }
    }
    let n = buf.len();
    *out = String::from_utf8_lossy(&buf).into_owned();
    Ok(n)
}

/// We heard a device announce itself: connect to it, and be the TLS server.
pub async fn connect_to(d: Arc<Daemon>, ip: IpAddr, port: u16, device_id: String) {
    let Ok(Ok(mut tcp)) = tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(SocketAddr::new(ip, port))).await else { return };
    keepalive(&tcp);
    if tcp.write_all(d.identity_packet(None, Some(&device_id)).line().as_bytes()).await.is_err() { return; }
    let Ok(acceptor) = acceptor(&d.me) else { return };
    let Ok(Ok(tls)) = tokio::time::timeout(Duration::from_secs(10), acceptor.accept(tcp)).await else { return };
    let cert = tls.get_ref().1.peer_certificates().and_then(|c| c.first()).map(|c| c.to_vec());
    secure(d, Box::new(tls), cert, device_id, ip).await;
}

/// A device connected to us: its identity first, then TLS with us as the client.
async fn accepted(d: Arc<Daemon>, mut tcp: TcpStream, ip: IpAddr) {
    keepalive(&tcp);
    let Ok(Some(line)) = tokio::time::timeout(Duration::from_secs(5), read_line_raw(&mut tcp, MAX_IDENTITY)).await else { return };
    let Some(p) = Packet::parse(&line) else { return };
    if p.kind != "kdeconnect.identity" { return; }
    let Some(id) = p.str("deviceId").map(String::from) else { return };
    if id == d.me.device_id || !crate::identity::valid_device_id(&id) { return; }
    if let Some(t) = p.str("targetDeviceId") { if t != d.me.device_id { return; } }
    if p.int("protocolVersion").unwrap_or(0) < packet::PROTOCOL_VERSION { d.note_old_protocol(&id, p.str("deviceName").unwrap_or("")); return; }
    if !d.may_accept(&id) { return; }
    let Ok(connector) = connector(&d.me) else { return };
    let Ok(Ok(tls)) = tokio::time::timeout(Duration::from_secs(10), connector.connect(server_name(&id), tcp)).await else { return };
    let cert = tls.get_ref().1.peer_certificates().and_then(|c| c.first()).map(|c| c.to_vec());
    secure(d, Box::new(tls), cert, id, ip).await;
}

/// Broadcast sockets need SO_BROADCAST; the port is shared (SO_REUSEADDR) in case KDE Connect itself also runs.
fn udp_socket(port: u16) -> std::io::Result<UdpSocket> {
    let s = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    s.set_reuse_address(true)?;
    s.set_broadcast(true)?;
    s.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)).into())?;
    s.set_nonblocking(true)?;
    UdpSocket::from_std(s.into())
}

/// Our local networks' broadcast addresses (and the general one).
pub fn broadcast_targets() -> Vec<Ipv4Addr> {
    let mut out = vec![Ipv4Addr::BROADCAST];
    for i in if_addrs::get_if_addrs().unwrap_or_default() {
        if i.is_loopback() { continue; }
        if let if_addrs::IfAddr::V4(a) = i.addr {
            if !crate::is_local_ip(IpAddr::V4(a.ip)) { continue; }
            let b = a.broadcast.unwrap_or_else(|| Ipv4Addr::from(u32::from(a.ip) | !u32::from(a.netmask)));
            if !out.contains(&b) { out.push(b); }
        }
    }
    out
}

/// Announces us (now, when asked, every minute, and to paired phones' last addresses directly), listens for
/// others' announcements, and accepts connections.
pub async fn run(d: Arc<Daemon>, mut stop: watch::Receiver<bool>) -> Result<(), String> {
    let mut listener = None;
    for p in packet::MIN_TCP_PORT..=packet::MAX_TCP_PORT {
        if let Ok(l) = TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, p))).await { listener = Some((l, p)); break; }
    }
    let (listener, tcp_port) = listener.ok_or("No free port in 1716-1764 for Hearth Connect")?;
    d.set_tcp_port(tcp_port);
    let udp = Arc::new(udp_socket(packet::UDP_PORT).map_err(|e| format!("Can't listen for phones on UDP {}: {e}", packet::UDP_PORT))?);

    // Accepting connections.
    {
        let (d, mut stop) = (d.clone(), stop.clone());
        tokio::spawn(async move {
            loop {
                let (tcp, addr) = tokio::select! {
                    r = listener.accept() => match r { Ok(x) => x, Err(_) => { tokio::time::sleep(Duration::from_millis(200)).await; continue; } },
                    _ = stop.changed() => return,
                };
                let ip = crate::canonical(addr.ip());
                if !crate::is_local_ip(ip) { continue; }
                tokio::spawn(accepted(d.clone(), tcp, ip));
            }
        });
    }
    // Announcing.
    {
        let (d, udp, mut stop) = (d.clone(), udp.clone(), stop.clone());
        tokio::spawn(async move {
            loop {
                let line = d.identity_packet(Some(tcp_port), None).line();
                for b in broadcast_targets() { let _ = udp.send_to(line.as_bytes(), SocketAddr::from((b, packet::UDP_PORT))).await; }
                for ip in d.addresses_to_try() { let _ = udp.send_to(line.as_bytes(), SocketAddr::new(ip, packet::UDP_PORT)).await; }
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(60)) => {}
                    _ = d.announce.notified() => {}
                    _ = stop.changed() => return,
                }
            }
        });
    }
    // Hearing others.
    let mut buf = vec![0u8; 65536];
    loop {
        let (n, from) = tokio::select! {
            r = udp.recv_from(&mut buf) => match r { Ok(x) => x, Err(_) => { tokio::time::sleep(Duration::from_millis(200)).await; continue; } },
            _ = stop.changed() => return Ok(()),
        };
        let ip = crate::canonical(from.ip());
        if !crate::is_local_ip(ip) { continue; }
        let Ok(text) = std::str::from_utf8(&buf[..n]) else { continue };
        let Some(p) = Packet::parse(text) else { continue };
        if p.kind != "kdeconnect.identity" { continue; }
        let Some(id) = p.str("deviceId").map(String::from) else { continue };
        if id == d.me.device_id || !crate::identity::valid_device_id(&id) { continue; }
        if p.int("protocolVersion").unwrap_or(0) < packet::PROTOCOL_VERSION { d.note_old_protocol(&id, p.str("deviceName").unwrap_or("")); continue; }
        let port = p.int("tcpPort").and_then(|x| u16::try_from(x).ok()).unwrap_or(0);
        if !(packet::MIN_TCP_PORT..=packet::MAX_TCP_PORT).contains(&port) { continue; }
        if !d.may_connect(&id) { continue; }
        tokio::spawn(connect_to(d.clone(), ip, port, id));
    }
}

/// Sends a file to a device: listen on 1739-1764, say the port in the packet, then serve the file (as the TLS
/// server) to the device when it connects, checking it's the paired certificate. Progress through `progress`.
pub async fn upload(d: &Arc<Daemon>, device_id: &str, mut packet: Packet, path: Option<std::path::PathBuf>, data: Vec<u8>, progress: impl Fn(u64)) -> Result<(), String> {
    let (ip, cert) = d.link_of(device_id).ok_or("The phone isn't connected")?;
    let size: u64 = match &path { Some(p) => tokio::fs::metadata(p).await.map_err(|e| e.to_string())?.len(), None => data.len() as u64 };
    let mut listener = None;
    for p in packet::MIN_PAYLOAD_PORT..=packet::MAX_PAYLOAD_PORT {
        if let Ok(l) = TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, p))).await { listener = Some((l, p)); break; }
    }
    let (listener, port) = listener.ok_or("No free port in 1739-1764 to send from")?;
    packet.payload_size = Some(size as i64);
    packet.payload_transfer_info = Some(serde_json::json!({"port": port}));
    d.send(device_id, &packet)?;
    // KDE Connect's own wait: 30 seconds for the other side to come and fetch it.
    let (tcp, addr) = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let (s, a) = listener.accept().await.map_err(|e| e.to_string())?;
            if crate::canonical(a.ip()) == ip { return Ok::<_, String>((s, a)); }
        }
    }).await.map_err(|_| "The phone didn't come to fetch the file".to_string())??;
    let _ = addr;
    keepalive(&tcp);
    let mut tls = tokio::time::timeout(Duration::from_secs(10), acceptor(&d.me)?.accept(tcp)).await.map_err(|_| "TLS took too long".to_string())?.map_err(|e| e.to_string())?;
    let theirs = tls.get_ref().1.peer_certificates().and_then(|c| c.first()).map(|c| c.to_vec());
    if theirs.as_deref() != Some(cert.as_slice()) { return Err("That wasn't the paired phone".into()); }
    let mut sent = 0u64;
    match path {
        Some(p) => {
            let mut f = tokio::fs::File::open(&p).await.map_err(|e| e.to_string())?;
            let mut buf = vec![0u8; 256 * 1024];
            loop {
                let n = f.read(&mut buf).await.map_err(|e| e.to_string())?;
                if n == 0 { break; }
                tls.write_all(&buf[..n]).await.map_err(|e| format!("The transfer broke: {e}"))?;
                sent += n as u64;
                progress(sent);
            }
        }
        None => { tls.write_all(&data).await.map_err(|e| e.to_string())?; progress(data.len() as u64); }
    }
    tls.flush().await.map_err(|e| e.to_string())?;
    let _ = tls.shutdown().await;
    Ok(())
}

/// Fetches a file a device offers (it listens on `port`), into `dest`. The device must be the paired certificate.
pub async fn download(d: &Arc<Daemon>, device_id: &str, port: u16, size: i64, dest: &std::path::Path, progress: impl Fn(u64)) -> Result<u64, String> {
    let (ip, cert) = d.link_of(device_id).ok_or("The phone isn't connected")?;
    let tcp = tokio::time::timeout(Duration::from_secs(10), TcpStream::connect(SocketAddr::new(ip, port))).await.map_err(|_| "The phone didn't answer".to_string())?.map_err(|e| e.to_string())?;
    keepalive(&tcp);
    let mut tls = tokio::time::timeout(Duration::from_secs(10), connector(&d.me)?.connect(server_name(device_id), tcp)).await.map_err(|_| "TLS took too long".to_string())?.map_err(|e| e.to_string())?;
    let theirs = tls.get_ref().1.peer_certificates().and_then(|c| c.first()).map(|c| c.to_vec());
    if theirs.as_deref() != Some(cert.as_slice()) { return Err("That wasn't the paired phone".into()); }
    let mut f = tokio::fs::File::create(dest).await.map_err(|e| format!("{}: {e}", dest.display()))?;
    let mut got = 0u64;
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        if size >= 0 && got >= size as u64 { break; }
        let want = if size >= 0 { ((size as u64 - got) as usize).min(buf.len()) } else { buf.len() };
        let n = match tokio::time::timeout(Duration::from_secs(60), tls.read(&mut buf[..want])).await {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => { if size < 0 { break; } return Err(format!("The transfer broke: {e}")); }
            Err(_) => return Err("The transfer stalled".into()),
        };
        if n == 0 { break; }
        f.write_all(&buf[..n]).await.map_err(|e| e.to_string())?;
        got += n as u64;
        progress(got);
    }
    f.flush().await.map_err(|e| e.to_string())?;
    if size >= 0 && got != size as u64 { return Err(format!("Only {got} of {size} bytes arrived")); }
    Ok(got)
}
