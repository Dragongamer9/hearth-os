//! Family and friends with Moonlight (the owner, Nov 20, 2026: "add the ability to connect to family and friends pc
//! with moonlight have it a deal they have to enable"; docs/HEARTH-LINK.md "Family and friends with Moonlight").
//!
//! **On the PC being connected to** (a friend's or a family member's): Settings › Gaming › Family and friends, "Let
//! family and friends connect (Moonlight)". Off until that person turns it on; turning it on says plainly what it
//! allows (seeing and controlling the screen, hearing the sound). Then:
//! - **Who can connect**: only the people they add, each with a one-time code (Hearth Link's pairing: 6 digits on the
//!   same network, a 10-character short code or its QR code from anywhere). Already paired devices (Dad's PC) keep
//!   whose they are; a new one is paired as a friend's device. Each can be removed any time.
//! - **Ask every time** (the default) or **Always allow**, per person: "Chris wants to connect: Allow / Deny" on this
//!   PC's screen (hearth-family-app --ask, or a notification with buttons where that app isn't), 60 seconds to answer.
//! - **What they get**: Whole desktop (Sunshine's Desktop) or Games only (Steam Big Picture; Sunshine's prep command
//!   refuses Desktop for them: hearth-remote-desktop session start reads this module's session file).
//! - **Time limits**: none, 30 minutes, 1, 2 or 3 hours per person; the session ends by itself at the limit.
//! - **While someone is connected**: a notice that stays on screen (and in the taskbar) with **Stop sharing now**
//!   (hearth-family-app --notice), a notification, and Game Mode's Quick Access line (the session file).
//! - **Off** ends every session at once (its connections are closed) and nobody new is let in.
//!
//! **On the connecting PC** (the owner's): Family and friends lists each PC that lets this one connect (online or not,
//! asked only when the list opens: no polling), **Connect** and **Remove**.
//!
//! **How the picture gets there**: no router port, no UPnP, no Tailscale needed. Moonlight's own connections go
//! through Hearth Link's connection to that PC (iroh: QUIC with TLS 1.3, each device's key pinned at pairing,
//! direct or hole punched, the relay when nothing else works), on its own protocol (ALPN `hearth/link-moonlight/1`),
//! which is only answered while that person's PC has said yes to this one (a grant), and only from paired devices.
//! - Moonlight runs here in a private network of its own (a user and network namespace: only `lo`, so Moonlight
//!   reaches nothing but the tunnel and this PC's own Sunshine, if it has one, is never in the way). Hearth Link
//!   opens Sunshine's ports there (TCP HTTPS, HTTP, RTSP; UDP video, control, audio) before Moonlight starts and
//!   carries each TCP connection on a QUIC stream and each UDP packet in QUIC datagrams (split in up to 8 parts
//!   when the path's datagrams are smaller, and dropped as a whole if a part is lost, as UDP would be).
//! - On that PC, Sunshine only takes connections from the PC itself (Hearth's nftables rule, `remote-fw local`);
//!   the tunnel's end connects to it on 127.0.0.1. Sunshine only runs while someone is connected (or while "Use this
//!   PC from anywhere" or Remote play has it on anyway), so nothing more is listening at rest than before.
//! - Moonlight still pairs with that Sunshine (its certificate): Hearth makes a one-time PIN, gives it to Moonlight
//!   here (`moonlight pair HOST --pin`, in its environment, never on screen or in a log) and sends it over Hearth
//!   Link to that PC's Sunshine. No password anywhere.
//!
//! Settings: ~/.config/nimbus/family-moonlight.json (`on`, `people: {deviceId: {name, always, what, limitMinutes}}`),
//! written only when the person changes something, through settings_file.rs (atomic, .bak, merged; a file that can't
//! be read is never saved over). Device ids aren't secrets; the keys stay in Hearth Link's own store (identity.key,
//! peers.json). Requests between devices (any relation; each checks for itself): moonlight.hello, moonlight.offer,
//! moonlight.revoke, moonlight.ask, moonlight.pin, moonlight.end.

use crate::node::Node;
use bytes::Bytes;
use iroh::endpoint::{Connection, RecvStream, SendStream, VarInt};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The tunnel's own QUIC protocol (answered only while a grant is on: node.rs on_conn).
pub const ALPN: &[u8] = b"hearth/link-moonlight/1";
/// Requests between devices for this (family.rs lets them through for every relation; each handler checks).
pub const OPS: &[&str] = &["moonlight.hello", "moonlight.offer", "moonlight.revoke", "moonlight.ask", "moonlight.pin", "moonlight.end"];
pub fn is_op(op: &str) -> bool { OPS.contains(&op) }

/// Sunshine's port (its `port` setting) and the others from it: TCP HTTPS (-5), HTTP (0), RTSP (+21); UDP video
/// (+9), control (+10), audio (+11).
pub const BASE: u16 = 47989;
const TCP_OFF: [i32; 3] = [-5, 0, 21];
const UDP_OFF: [i32; 3] = [9, 10, 11];
pub fn ports(base: u16) -> ([u16; 3], [u16; 3]) {
    let at = |o: i32| (base as i32 + o).clamp(1, 65535) as u16;
    ([at(TCP_OFF[0]), at(TCP_OFF[1]), at(TCP_OFF[2])], [at(UDP_OFF[0]), at(UDP_OFF[1]), at(UDP_OFF[2])])
}

/// The time limits a person can pick (minutes; 0: none).
pub const LIMITS: [u32; 5] = [0, 30, 60, 120, 180];
/// What they get.
pub const WHAT: [&str; 2] = ["desktop", "games"];
/// Sunshine's app for each.
pub fn app_for(what: &str) -> &'static str { if what == "games" { "Steam Big Picture" } else { "Desktop" } }
/// How long the person at the PC has to answer "wants to connect".
const ANSWER_WAIT: Duration = Duration::from_secs(60);
/// A yes the other PC doesn't use (Moonlight never connects) ends after this.
const UNUSED_GRANT: Duration = Duration::from_secs(120);

/// A minute, in milliseconds ($HEARTH_LINK_MOON_MINUTE_MS for the tests' time limits).
fn minute_ms() -> u64 { std::env::var("HEARTH_LINK_MOON_MINUTE_MS").ok().and_then(|v| v.parse().ok()).filter(|v| *v > 0).unwrap_or(60_000) }

// ------------------------------------------------------------------ the person's choices (a settings file)

pub fn settings_path() -> PathBuf {
    std::env::var_os("HEARTH_FAMILY_MOONLIGHT").map(PathBuf::from).unwrap_or_else(|| hearth_core::paths::config_dir().join("family-moonlight.json"))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Person { pub name: String, pub always: bool, pub what: String, pub limit: u32 }

#[derive(Debug, Clone, PartialEq)]
pub struct Share { pub on: bool, pub people: Vec<(String, Person)>, pub readable: bool }

impl Default for Share { fn default() -> Self { Share { on: false, people: Vec::new(), readable: true } } }

impl Share {
    pub fn person(&self, id: &str) -> Option<&Person> { self.people.iter().find(|(i, _)| i == id).map(|(_, p)| p) }
}

pub fn valid_id(id: &str) -> bool { id.len() == 64 && id.chars().all(|c| c.is_ascii_hexdigit()) }
pub fn clean_name(n: &str) -> String { n.chars().filter(|c| !c.is_control()).take(40).collect::<String>().trim().to_string() }

/// The file's contents as choices: anything that isn't one of the choices is read as its default (and never saved
/// back as one: saving changes only what the person changed).
pub fn parse(m: &Map<String, Value>) -> Share {
    let mut people = Vec::new();
    if let Some(Value::Object(ps)) = m.get("people") {
        for (id, v) in ps {
            if !valid_id(id) { continue; }
            let what = v["what"].as_str().filter(|w| WHAT.contains(w)).unwrap_or("desktop").to_string();
            let limit = v["limitMinutes"].as_u64().map(|x| x as u32).filter(|x| LIMITS.contains(x)).unwrap_or(0);
            people.push((id.clone(), Person { name: clean_name(v["name"].as_str().unwrap_or("")), always: v["always"] == true, what, limit }));
        }
    }
    Share { on: m.get("on") == Some(&Value::Bool(true)), people, readable: true }
}

pub fn load() -> Share {
    use hearth_core::settings_file::{read, Read};
    match read(&settings_path()) {
        Read::Missing => Share::default(),
        Read::Ok(m) | Read::Restored(m) => parse(&m),
        Read::Unreadable => Share { readable: false, ..Share::default() },
    }
}

/// Changes the file (merged under its lock, .bak kept). Refused when it can't be read: defaults are never saved over it.
pub fn save(change: impl FnOnce(&mut Map<String, Value>)) -> Result<(), String> {
    if !load().readable { return Err("Your Family and friends settings couldn't be read, so nothing was changed".into()); }
    hearth_core::settings_file::update_json(&settings_path(), change).map(|_| ()).map_err(|e| format!("Couldn't save: {e}"))
}

pub fn person_json(p: &Person) -> Value { json!({"name": p.name, "always": p.always, "what": p.what, "limitMinutes": p.limit}) }

/// One person's choices changed, checked: `{always?, what?, limitMinutes?}`.
pub fn change_person(m: &mut Map<String, Value>, id: &str, a: &Value) -> Result<(), String> {
    let ps = m.entry("people").or_insert_with(|| json!({}));
    if !ps.is_object() { *ps = json!({}); }
    let p = ps.get_mut(id).filter(|p| p.is_object()).ok_or("That person isn't on your list")?;
    if let Some(b) = a["always"].as_bool() { p["always"] = json!(b); }
    if let Some(w) = a["what"].as_str() { if !WHAT.contains(&w) { return Err("Whole desktop or Games only".into()); } p["what"] = json!(w); }
    if let Some(l) = a["limitMinutes"].as_u64() { if !LIMITS.contains(&(l as u32)) { return Err("No limit, 30 minutes, 1, 2 or 3 hours".into()); } p["limitMinutes"] = json!(l); }
    Ok(())
}

/// What a person is called in Sunshine's list of paired Moonlight devices, so removing them unpairs that device.
pub fn client_key(peer: &str) -> String { format!("Hearth {}", peer.get(..8).unwrap_or(peer)) }

/// The address Moonlight uses for that PC in its private network: one of its own per PC (127.77.x.y), so Moonlight
/// keeps each PC apart.
pub fn host_addr(peer: &str) -> Ipv4Addr {
    let h = blake3::hash(peer.as_bytes());
    let b = h.as_bytes();
    Ipv4Addr::new(127, 77, b[0].max(1), b[1].max(1))
}

/// "allow", "always" or "deny" from what the question printed.
pub fn answer_word(out: &str) -> &'static str {
    match out.lines().rev().map(str::trim).find(|l| !l.is_empty()).unwrap_or("") {
        "allow" | "Allow" => "allow",
        "always" | "Always" => "always",
        _ => "deny",
    }
}

// ------------------------------------------------------------------ UDP packets in QUIC datagrams

/// A packet as datagrams of at most `max` bytes: 4 bytes in front (which port, a number, part and parts), split in
/// up to 8 parts. None: too big even for that (it's dropped, as a network would).
pub fn frag(idx: u8, seq: u16, data: &[u8], max: usize) -> Option<Vec<Vec<u8>>> {
    let room = max.saturating_sub(4);
    if room == 0 { return None; }
    let parts = data.len().div_ceil(room).max(1);
    if parts > 8 { return None; }
    let per = data.len().div_ceil(parts).max(1);
    let s = seq.to_be_bytes();
    Some((0..parts).map(|i| {
        let chunk = &data[(i * per).min(data.len())..((i + 1) * per).min(data.len())];
        let mut d = Vec::with_capacity(4 + chunk.len());
        d.extend_from_slice(&[idx, s[0], s[1], ((i as u8) << 4) | parts as u8]);
        d.extend_from_slice(chunk);
        d
    }).collect())
}

/// Puts split packets back together; a packet whose parts don't all come within half a second is dropped.
#[derive(Default)]
pub struct Reasm { parts: HashMap<(u8, u16), (Instant, Vec<Option<Vec<u8>>>)> }

impl Reasm {
    pub fn take(&mut self, d: &[u8]) -> Option<(u8, Vec<u8>)> {
        if d.len() < 4 { return None; }
        let (idx, seq, part, total) = (d[0], u16::from_be_bytes([d[1], d[2]]), (d[3] >> 4) as usize, (d[3] & 15) as usize);
        if total == 0 || total > 8 || part >= total { return None; }
        if total == 1 { return Some((idx, d[4..].to_vec())); }
        let now = Instant::now();
        if self.parts.len() > 64 { self.parts.retain(|_, (t, _)| now.duration_since(*t) < Duration::from_millis(500)); }
        let e = self.parts.entry((idx, seq)).or_insert_with(|| (now, vec![None; total]));
        if e.1.len() != total { *e = (now, vec![None; total]); }
        e.1[part] = Some(d[4..].to_vec());
        if e.1.iter().all(Option::is_some) {
            let (_, v) = self.parts.remove(&(idx, seq))?;
            return Some((idx, v.into_iter().flatten().flatten().collect()));
        }
        None
    }
}

fn send_packet(c: &Connection, idx: u8, seq: &mut u16, data: &[u8]) {
    let max = c.max_datagram_size().unwrap_or(1200);
    *seq = seq.wrapping_add(1);
    if let Some(ds) = frag(idx, *seq, data, max) {
        for d in ds { let _ = c.send_datagram(Bytes::from(d)); }
    }
}

/// One TCP connection on one QUIC stream, both ways, until either side ends.
async fn pump(t: tokio::net::TcpStream, mut s: SendStream, mut r: RecvStream) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let _ = t.set_nodelay(true);
    let (mut tr, mut tw) = t.into_split();
    let up = async move {
        let mut b = vec![0u8; 32 << 10];
        loop {
            match tr.read(&mut b).await { Ok(0) | Err(_) => break, Ok(n) => if s.write_all(&b[..n]).await.is_err() { break } }
        }
        let _ = s.finish();
    };
    let down = async move {
        let mut b = vec![0u8; 32 << 10];
        loop {
            match r.read(&mut b).await { Ok(Some(n)) => if tw.write_all(&b[..n]).await.is_err() { break }, _ => break }
        }
        let _ = tw.shutdown().await;
    };
    tokio::join!(up, down);
}

/// The tunnel's end on the PC being connected to: each stream to Sunshine's TCP port it names, datagrams to and
/// from its UDP ports. Returns when the connection ends.
pub async fn serve_tunnel(conn: Connection, base: u16) {
    let (tcp, udp) = ports(base);
    let mut socks = Vec::new();
    for p in udp {
        let Ok(s) = tokio::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await else { return };
        if s.connect((Ipv4Addr::LOCALHOST, p)).await.is_err() { return; }
        socks.push(Arc::new(s));
    }
    let mut tasks = tokio::task::JoinSet::new();
    {
        let (c, socks) = (conn.clone(), socks.clone());
        tasks.spawn(async move {
            let mut re = Reasm::default();
            while let Ok(d) = c.read_datagram().await {
                if let Some((i, data)) = re.take(&d) { if let Some(s) = socks.get(i as usize) { let _ = s.send(&data).await; } }
            }
        });
    }
    for (i, s) in socks.iter().enumerate() {
        let (c, s) = (conn.clone(), s.clone());
        tasks.spawn(async move {
            let mut buf = vec![0u8; 65536];
            let mut seq = 0u16;
            loop {
                tokio::select! {
                    r = s.recv(&mut buf) => match r {
                        Ok(n) => send_packet(&c, i as u8, &mut seq, &buf[..n]),
                        // Sunshine not listening yet (a refused packet comes back as an error): carry on.
                        Err(_) => tokio::time::sleep(Duration::from_millis(5)).await,
                    },
                    _ = c.closed() => break,
                }
            }
        });
    }
    while let Ok((s, mut r)) = conn.accept_bi().await {
        tokio::spawn(async move {
            let mut h = [0u8; 2];
            if r.read_exact(&mut h).await.is_err() || h[0] != b'T' || h[1] as usize >= tcp.len() { return; }
            let Ok(t) = tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, tcp[h[1] as usize])).await else { return };
            pump(t, s, r).await;
        });
    }
    tasks.abort_all();
}

/// The tunnel's end on the connecting PC: Moonlight's connections (in its private network) to that PC.
pub async fn run_tunnel(conn: Connection, listeners: Vec<tokio::net::TcpListener>, udps: Vec<tokio::net::UdpSocket>) {
    let mut tasks = tokio::task::JoinSet::new();
    for (i, l) in listeners.into_iter().enumerate() {
        let c = conn.clone();
        tasks.spawn(async move {
            loop {
                tokio::select! {
                    a = l.accept() => {
                        let Ok((t, _)) = a else { continue };
                        let c2 = c.clone();
                        tokio::spawn(async move {
                            let Ok((mut s, r)) = c2.open_bi().await else { return };
                            if s.write_all(&[b'T', i as u8]).await.is_err() { return; }
                            pump(t, s, r).await;
                        });
                    }
                    _ = c.closed() => break,
                }
            }
        });
    }
    let socks: Vec<Arc<tokio::net::UdpSocket>> = udps.into_iter().map(Arc::new).collect();
    let back: Arc<Mutex<[Option<SocketAddr>; 3]>> = Arc::new(Mutex::new([None; 3]));
    for (i, s) in socks.iter().enumerate() {
        let (c, s, back) = (conn.clone(), s.clone(), back.clone());
        tasks.spawn(async move {
            let mut buf = vec![0u8; 65536];
            let mut seq = 0u16;
            loop {
                tokio::select! {
                    r = s.recv_from(&mut buf) => match r {
                        Ok((n, from)) => { crate::lock(&back)[i] = Some(from); send_packet(&c, i as u8, &mut seq, &buf[..n]); }
                        Err(_) => tokio::time::sleep(Duration::from_millis(5)).await,
                    },
                    _ = c.closed() => break,
                }
            }
        });
    }
    let mut re = Reasm::default();
    while let Ok(d) = conn.read_datagram().await {
        if let Some((i, data)) = re.take(&d) {
            let to = crate::lock(&back).get(i as usize).copied().flatten();
            if let (Some(s), Some(to)) = (socks.get(i as usize), to) { let _ = s.send_to(&data, to).await; }
        }
    }
    tasks.abort_all();
}

// ------------------------------------------------------------------ Moonlight's private network (connecting PC)

/// Moonlight, started in a network of its own with Sunshine's ports open at `addr` (sockets this process holds).
pub struct Private { pub child: std::process::Child, pub tcp: Vec<std::net::TcpListener>, pub udp: Vec<std::net::UdpSocket> }

/// Starts `prog` in a new user and network namespace (only `lo`, brought up) with Sunshine's ports bound at `addr`
/// there before it runs; the sockets come back to this process over a socket pair. Nothing is allocated between
/// fork and exec (the maps are written before).
pub fn spawn_private(prog: &[String], envs: &[(String, String)], addr: Ipv4Addr, base: u16) -> Result<Private, String> {
    use std::os::unix::process::CommandExt;
    if prog.is_empty() { return Err("Nothing to start".into()); }
    let (tcp, udp) = ports(base);
    let mut sv = [0i32; 2];
    if unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0, sv.as_mut_ptr()) } != 0 {
        return Err(format!("socketpair: {}", std::io::Error::last_os_error()));
    }
    let (mine, theirs) = (sv[0], sv[1]);
    let uid = unsafe { libc::geteuid() };
    let gid = unsafe { libc::getegid() };
    let uid_map = format!("{uid} {uid} 1\n").into_bytes();
    let gid_map = format!("{gid} {gid} 1\n").into_bytes();
    let ip_be = u32::from(addr).to_be();
    let mut cmd = std::process::Command::new(&prog[0]);
    cmd.args(&prog[1..]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    for (k, v) in envs { cmd.env(k, v); }
    unsafe { cmd.pre_exec(move || enter_private(theirs, &uid_map, &gid_map, ip_be, tcp, udp)); }
    let child = cmd.spawn();
    unsafe { libc::close(theirs) };
    let child = match child {
        Ok(c) => c,
        Err(e) => {
            unsafe { libc::close(mine) };
            return Err(if e.raw_os_error() == Some(libc::EPERM) || e.raw_os_error() == Some(libc::EINVAL) || e.raw_os_error() == Some(libc::ENOSPC) {
                "This PC doesn't let apps have a private network (user namespaces are off), which Moonlight needs here".to_string()
            } else if e.raw_os_error() == Some(libc::EADDRINUSE) {
                "Moonlight's ports were taken in its own network (that shouldn't happen): try again".to_string()
            } else { format!("Moonlight couldn't start: {e}") });
        }
    };
    let fds = recv_fds(mine, 6);
    unsafe { libc::close(mine) };
    let fds = fds.map_err(|e| format!("Moonlight's network: {e}"))?;
    use std::os::fd::FromRawFd;
    let tcp = fds[..3].iter().map(|f| unsafe { std::net::TcpListener::from_raw_fd(*f) }).collect();
    let udp = fds[3..].iter().map(|f| unsafe { std::net::UdpSocket::from_raw_fd(*f) }).collect();
    Ok(Private { child, tcp, udp })
}

/// In the child, before exec.
fn enter_private(sock: i32, uid_map: &[u8], gid_map: &[u8], ip_be: u32, tcp: [u16; 3], udp: [u16; 3]) -> std::io::Result<()> {
    let err = std::io::Error::last_os_error;
    unsafe {
        if libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNET) != 0 { return Err(err()); }
        let _ = write_proc(b"/proc/self/setgroups\0", b"deny");
        write_proc(b"/proc/self/uid_map\0", uid_map)?;
        write_proc(b"/proc/self/gid_map\0", gid_map)?;
        let s = libc::socket(libc::AF_INET, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0);
        if s < 0 { return Err(err()); }
        let mut ifr: libc::ifreq = std::mem::zeroed();
        ifr.ifr_name[0] = b'l' as libc::c_char;
        ifr.ifr_name[1] = b'o' as libc::c_char;
        if libc::ioctl(s, libc::SIOCGIFFLAGS as _, &mut ifr as *mut libc::ifreq) != 0 { let e = err(); libc::close(s); return Err(e); }
        ifr.ifr_ifru.ifru_flags |= (libc::IFF_UP | libc::IFF_RUNNING) as libc::c_short;
        if libc::ioctl(s, libc::SIOCSIFFLAGS as _, &ifr as *const libc::ifreq) != 0 { let e = err(); libc::close(s); return Err(e); }
        libc::close(s);
        let mut fds = [-1i32; 6];
        for i in 0..3 {
            fds[i] = bind_sock(libc::SOCK_STREAM, ip_be, tcp[i])?;
            if libc::listen(fds[i], 64) != 0 { return Err(err()); }
        }
        for i in 0..3 { fds[3 + i] = bind_sock(libc::SOCK_DGRAM, ip_be, udp[i])?; }
        send_fds(sock, &fds)?;
        for f in fds { libc::close(f); }
    }
    Ok(())
}

unsafe fn write_proc(path: &[u8], data: &[u8]) -> std::io::Result<()> {
    let fd = libc::open(path.as_ptr() as *const libc::c_char, libc::O_WRONLY | libc::O_CLOEXEC);
    if fd < 0 { return Err(std::io::Error::last_os_error()); }
    let n = libc::write(fd, data.as_ptr() as *const libc::c_void, data.len());
    let e = std::io::Error::last_os_error();
    libc::close(fd);
    if n != data.len() as isize { return Err(e); }
    Ok(())
}

unsafe fn bind_sock(kind: i32, ip_be: u32, port: u16) -> std::io::Result<i32> {
    let s = libc::socket(libc::AF_INET, kind | libc::SOCK_CLOEXEC, 0);
    if s < 0 { return Err(std::io::Error::last_os_error()); }
    let one: libc::c_int = 1;
    libc::setsockopt(s, libc::SOL_SOCKET, libc::SO_REUSEADDR, &one as *const _ as *const libc::c_void, std::mem::size_of::<libc::c_int>() as libc::socklen_t);
    let mut sa: libc::sockaddr_in = std::mem::zeroed();
    sa.sin_family = libc::AF_INET as libc::sa_family_t;
    sa.sin_port = port.to_be();
    sa.sin_addr = libc::in_addr { s_addr: ip_be };
    if libc::bind(s, &sa as *const _ as *const libc::sockaddr, std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t) != 0 {
        let e = std::io::Error::last_os_error();
        libc::close(s);
        return Err(e);
    }
    Ok(s)
}

unsafe fn send_fds(sock: i32, fds: &[i32; 6]) -> std::io::Result<()> {
    let mut byte = [0u8; 1];
    let mut iov = libc::iovec { iov_base: byte.as_mut_ptr() as *mut libc::c_void, iov_len: 1 };
    let mut cbuf = [0u64; 16];
    let mut msg: libc::msghdr = std::mem::zeroed();
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = cbuf.as_mut_ptr() as *mut libc::c_void;
    msg.msg_controllen = libc::CMSG_SPACE((6 * std::mem::size_of::<i32>()) as u32) as _;
    let c = libc::CMSG_FIRSTHDR(&msg);
    (*c).cmsg_level = libc::SOL_SOCKET;
    (*c).cmsg_type = libc::SCM_RIGHTS;
    (*c).cmsg_len = libc::CMSG_LEN((6 * std::mem::size_of::<i32>()) as u32) as _;
    std::ptr::copy_nonoverlapping(fds.as_ptr() as *const u8, libc::CMSG_DATA(c), 6 * std::mem::size_of::<i32>());
    if libc::sendmsg(sock, &msg, 0) < 0 { return Err(std::io::Error::last_os_error()); }
    Ok(())
}

fn recv_fds(sock: i32, n: usize) -> Result<Vec<i32>, String> {
    unsafe {
        let tv = libc::timeval { tv_sec: 5, tv_usec: 0 };
        libc::setsockopt(sock, libc::SOL_SOCKET, libc::SO_RCVTIMEO, &tv as *const _ as *const libc::c_void, std::mem::size_of::<libc::timeval>() as libc::socklen_t);
        let mut byte = [0u8; 1];
        let mut iov = libc::iovec { iov_base: byte.as_mut_ptr() as *mut libc::c_void, iov_len: 1 };
        let mut cbuf = [0u64; 16];
        let mut msg: libc::msghdr = std::mem::zeroed();
        msg.msg_iov = &mut iov;
        msg.msg_iovlen = 1;
        msg.msg_control = cbuf.as_mut_ptr() as *mut libc::c_void;
        msg.msg_controllen = std::mem::size_of_val(&cbuf) as _;
        if libc::recvmsg(sock, &mut msg, libc::MSG_CMSG_CLOEXEC) < 0 { return Err(std::io::Error::last_os_error().to_string()); }
        let c = libc::CMSG_FIRSTHDR(&msg);
        if c.is_null() || (*c).cmsg_type != libc::SCM_RIGHTS { return Err("no sockets came back".into()); }
        let got = ((*c).cmsg_len as usize - libc::CMSG_LEN(0) as usize) / std::mem::size_of::<i32>();
        let mut fds = vec![-1i32; got];
        std::ptr::copy_nonoverlapping(libc::CMSG_DATA(c), fds.as_mut_ptr() as *mut u8, got * std::mem::size_of::<i32>());
        if got != n { for f in fds { libc::close(f); } return Err("not every socket came back".into()); }
        Ok(fds)
    }
}

/// Moonlight on this PC: $HEARTH_MOONLIGHT (tests), `moonlight` (Moonlight Qt), or Flathub's.
pub fn moonlight_cmd() -> Option<Vec<String>> {
    if let Ok(p) = std::env::var("HEARTH_MOONLIGHT") { if !p.is_empty() { return Some(vec![p]); } }
    if let Some(p) = which("moonlight") { return Some(vec![p]); }
    let home = hearth_core::paths::home();
    let flat = ["/var/lib/flatpak/app/com.moonlight_stream.Moonlight".into(), home.join(".local/share/flatpak/app/com.moonlight_stream.Moonlight")];
    if flat.iter().any(|p: &PathBuf| p.exists()) {
        if let Some(f) = which("flatpak") { return Some(vec![f, "run".into(), "com.moonlight_stream.Moonlight".into()]); }
    }
    None
}

pub fn which(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(name)).find(|p| p.is_file()).map(|p| p.to_string_lossy().to_string())
}

/// `hearth-link moon-session HOST APP [BITRATE]`, run in Moonlight's private network: pairs Moonlight first when
/// $HEARTH_MOON_PIN is set (`moonlight pair HOST --pin`), then becomes `moonlight stream HOST APP`.
pub fn moon_session(args: &[String]) -> i32 {
    use std::os::unix::process::CommandExt;
    let (Some(host), Some(app)) = (args.first(), args.get(1)) else { eprintln!("usage: hearth-link moon-session HOST APP [BITRATE]"); return 2 };
    let Some(ml) = moonlight_cmd() else { eprintln!("Moonlight isn't installed"); return 3 };
    if let Ok(pin) = std::env::var("HEARTH_MOON_PIN") {
        std::env::remove_var("HEARTH_MOON_PIN");
        let mut c = std::process::Command::new(&ml[0]);
        c.args(&ml[1..]).args(["pair", host.as_str(), "--pin", pin.as_str()]).stdin(std::process::Stdio::null());
        let Ok(mut child) = c.spawn() else { return 3 };
        let t = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(s)) if s.success() => break,
                Ok(Some(_)) | Err(_) => return 4,
                Ok(None) if t.elapsed() > Duration::from_secs(90) => { let _ = child.kill(); let _ = child.wait(); return 4; }
                Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            }
        }
    }
    let mut c = std::process::Command::new(&ml[0]);
    c.args(&ml[1..]).args(["stream", host.as_str(), app.as_str()]);
    if let Some(b) = args.get(2).filter(|b| b.parse::<u32>().is_ok()) { c.args(["--bitrate", b.as_str()]); }
    let e = c.exec();
    eprintln!("Moonlight couldn't start: {e}");
    3
}

// ------------------------------------------------------------------ the service's side

/// Someone connected to this PC (or allowed to, until Moonlight does).
pub struct Grant {
    pub gen: u64,
    pub peer: String,
    pub name: String,
    pub what: String,
    pub until: u64,
    pub started: u64,
    pub port: u16,
    pub conns: Vec<Connection>,
    pub used: bool,
    pub notice: Option<std::process::Child>,
}

/// This PC connected to another (Moonlight running here).
pub struct Out { pub gen: u64, pub peer: String, pub name: String, pub conn: Option<Connection>, pub pid: u32, pub started: u64, pub until: u64, pub relay: bool }

#[derive(Default)]
pub struct Moon {
    /// An "Add a person" code is showing: the next device that pairs is let connect.
    pub invite: Mutex<Option<u64>>,
    pub grant: Mutex<Option<Grant>>,
    pub asking: AtomicBool,
    pub out: Mutex<Option<Out>>,
    pub gen: AtomicU64,
    /// The last thing that happened (for the screens): "Chris's PC said no", "The time limit ended the session".
    pub last: Mutex<Option<(u64, String)>>,
}

fn runtime_file() -> PathBuf { crate::socket_path().with_file_name("moonlight-session.json") }

fn grant_json(g: &Grant) -> Value {
    json!({"peer": g.peer, "who": g.name, "what": g.what, "until": g.until, "started": g.started, "connected": g.used})
}

/// The person at the PC answers "wants to connect": hearth-family-app's question ($HEARTH_FAMILY_ASK for tests), or
/// a notification with Allow and Deny where it isn't. ("deny", why) when nobody could be asked.
fn ask_person(who: &str, what: &str, limit: u32) -> (String, String) {
    let lim = limit.to_string();
    let app = std::env::var("HEARTH_FAMILY_ASK").ok().filter(|p| !p.is_empty()).or_else(|| which("hearth-family-app"));
    if let Some(app) = app {
        let args = ["--ask", "--name", who, "--what", what, "--limit", lim.as_str()];
        if let Some(out) = run_out(&app, &args, ANSWER_WAIT) { return (answer_word(&out).into(), String::new()); }
        return ("deny".into(), "Nobody answered on that PC".into());
    }
    if let Some(ns) = which("notify-send") {
        let body = format!("{who} wants to see and control this PC's {} with Moonlight, and hear its sound.", if what == "games" { "games" } else { "screen" });
        let title = format!("{who} wants to connect");
        let args = ["--wait", "-a", "Hearth", "-u", "critical", "-A", "allow=Allow", "-A", "deny=Deny", title.as_str(), body.as_str()];
        if let Some(out) = run_out(&ns, &args, ANSWER_WAIT) { return (answer_word(&out).into(), String::new()); }
        return ("deny".into(), "Nobody answered on that PC".into());
    }
    ("deny".into(), "That PC has no way to ask (no screen to show the question on)".into())
}

/// Runs a program and gives back what it printed, or None when it didn't finish in time (it's stopped).
fn run_out(prog: &str, args: &[&str], wait: Duration) -> Option<String> {
    use std::io::Read;
    let mut child = std::process::Command::new(prog).args(args).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null()).spawn().ok()?;
    let t = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if t.elapsed() > wait => { let _ = child.kill(); let _ = child.wait(); return None; }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => return None,
        }
    }
    let mut s = String::new();
    child.stdout.take()?.read_to_string(&mut s).ok()?;
    Some(s)
}

/// The notice that stays on screen while someone is connected (with Stop sharing now), and a notification.
fn show_notice(who: &str, what: &str, until: u64) -> Option<std::process::Child> {
    use std::process::Stdio;
    let app = std::env::var("HEARTH_FAMILY_NOTICE").ok().filter(|p| !p.is_empty()).or_else(|| which("hearth-family-app"));
    let ns = which("notify-send");
    if let Some(app) = app {
        if let Some(ns) = &ns {
            let _ = std::process::Command::new(ns).args(["-a", "Hearth", &format!("{who} is connected"), "They can see and control this PC with Moonlight. Stop sharing now is on the notice."])
                .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map(|mut c| std::thread::spawn(move || c.wait()));
        }
        let u = until.to_string();
        return std::process::Command::new(app).args(["--notice", "--name", who, "--what", what, "--until", u.as_str()])
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().ok();
    }
    // Without hearth-family-app: a notification that stays (critical) with Stop sharing now on it.
    let ns = ns?;
    let title = format!("{who} is connected");
    let body = format!("They can see and control this PC and hear its sound with Moonlight ({}).", if what == "games" { "games only" } else { "whole desktop" });
    let mut c = std::process::Command::new(ns).args(["--wait", "-a", "Hearth", "-u", "critical", "-A", "stop=Stop sharing now", title.as_str(), body.as_str()])
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let out = c.stdout.take();
    std::thread::spawn(move || {
        use std::io::Read;
        let mut s = String::new();
        if let Some(mut o) = out { let _ = o.read_to_string(&mut s); }
        if s.trim() == "stop" {
            if let Ok(me) = std::env::current_exe() { let _ = std::process::Command::new(me).args(["call", "moonlight.stop"]).stdin(Stdio::null()).stdout(Stdio::null()).status(); }
        }
    });
    Some(c)
}

fn helper(args: &[&str], secs: u64) -> Result<Value, String> { crate::family::run_timed(&crate::family::remote_helper(), args, secs) }

impl Node {
    fn moon_said(&self, s: impl Into<String>) { *crate::lock(&self.moon.last) = Some((crate::now_ms(), s.into())); }

    fn peer_name(&self, id: &str) -> String { crate::lock(&self.peers).get(id).map(|p| p.name.clone()).unwrap_or_else(|| "A paired device".into()) }

    /// After a pairing while "Add a person" showed its code: they may connect (asked every time, Whole desktop, no
    /// limit, until the person here changes that), and they're told.
    pub fn moon_after_pairing(self: &Arc<Self>, id: &str, name: &str) -> bool {
        let open = crate::lock(&self.moon.invite).take().map(|u| u > crate::now_ms()).unwrap_or(false);
        if !open { return false; }
        let (id2, name) = (id.to_string(), clean_name(name));
        let r = save(|m| {
            let ps = m.entry("people").or_insert_with(|| json!({}));
            if !ps.is_object() { *ps = json!({}); }
            if ps.get(&id2).is_none() { ps[&id2] = json!({"name": name, "always": false, "what": "desktop", "limitMinutes": 0}); }
        });
        match r {
            Ok(()) => eprintln!("hearth-link: moonlight: {} ({}) may connect now (asked every time)", name, id.get(..8).unwrap_or("")),
            Err(e) => { eprintln!("hearth-link: moonlight: {e}"); return true; }
        }
        let n = self.clone(); let id = id.to_string();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = tokio::time::timeout(Duration::from_secs(15), n.request(&id, &json!({"op": "moonlight.offer", "what": "desktop"}))).await;
        });
        true
    }

    /// Ends the session on this PC at once: its connections closed (Moonlight's picture stops), the notice gone,
    /// Sunshine's app closed and Sunshine stopped again if it ran only for this.
    pub fn moon_end(self: &Arc<Self>, why: &str) {
        let g = crate::lock(&self.moon.grant).take();
        let Some(mut g) = g else { return };
        for c in &g.conns { c.close(VarInt::from_u32(0), b"sharing stopped"); }
        if let Some(mut n) = g.notice.take() { let _ = n.kill(); std::thread::spawn(move || n.wait()); }
        let _ = std::fs::remove_file(runtime_file());
        eprintln!("hearth-link: moonlight: {} ({}): session ended: {why}", g.name, g.peer.get(..8).unwrap_or(""));
        self.moon_said(format!("{}: {why}", g.name));
        let peer = g.peer.clone();
        let n = self.clone();
        tokio::spawn(async move {
            let _ = tokio::task::spawn_blocking(|| helper(&["family", "idle"], 40)).await;
            let _ = tokio::time::timeout(Duration::from_secs(8), n.request(&peer, &json!({"op": "moonlight.end"}))).await;
        });
    }

    fn moon_end_if(self: &Arc<Self>, gen: u64, why: &str) {
        let same = crate::lock(&self.moon.grant).as_ref().map(|g| g.gen == gen).unwrap_or(false);
        if same { self.moon_end(why); }
    }

    /// A paired device's request (family.rs let it through: each one checks here).
    pub async fn moon_answer(self: Arc<Self>, peer: &str, op: &str, req: &Value) -> Result<Value, String> {
        match op {
            "moonlight.hello" => {
                let sh = load();
                let p = sh.person(peer);
                Ok(json!({"approved": p.is_some(), "sharing": sh.on && p.is_some(), "what": p.map(|p| p.what.clone()), "ask": p.map(|p| !p.always),
                          "limitMinutes": p.map(|p| p.limit), "name": self.name()}))
            }
            "moonlight.offer" => {
                let what = req["what"].as_str().filter(|w| WHAT.contains(w)).unwrap_or("desktop");
                let mut ps = crate::lock(&self.peers);
                let p = ps.get_mut(peer).ok_or("Not a paired device")?;
                let name = p.name.clone();
                if p.moonlight != what { p.moonlight = what.to_string(); ps.save()?; }
                eprintln!("hearth-link: moonlight: {name} lets this PC connect");
                Ok(json!({"ok": true}))
            }
            "moonlight.revoke" => {
                let mut ps = crate::lock(&self.peers);
                if let Some(p) = ps.get_mut(peer) { if !p.moonlight.is_empty() { p.moonlight.clear(); ps.save()?; } }
                Ok(json!({"ok": true}))
            }
            "moonlight.ask" => self.moon_ask(peer).await,
            "moonlight.pin" => {
                let pin: String = req["pin"].as_str().unwrap_or("").chars().filter(|c| c.is_ascii_digit()).collect();
                if pin.len() != 4 { return Err("Moonlight's PIN is 4 numbers".into()); }
                if !crate::lock(&self.moon.grant).as_ref().map(|g| g.peer == peer).unwrap_or(false) { return Err("That PC didn't say yes to this one right now".into()); }
                let key = client_key(peer);
                // Moonlight asks Sunshine first; the PIN only fits once it has (a few tries, a second apart).
                for i in 0..20 {
                    let (p, k) = (pin.clone(), key.clone());
                    match tokio::task::spawn_blocking(move || helper(&["pin", p.as_str(), k.as_str()], 45)).await.map_err(|e| e.to_string())? {
                        Ok(v) => { eprintln!("hearth-link: moonlight: {}: Moonlight paired with this PC's Sunshine", self.peer_name(peer)); return Ok(v); }
                        Err(e) if e.contains("No device is waiting") && i < 19 => tokio::time::sleep(Duration::from_secs(1)).await,
                        Err(e) => return Err(e),
                    }
                }
                Err("Moonlight didn't ask this PC's Sunshine to pair".into())
            }
            "moonlight.end" => {
                if crate::lock(&self.moon.grant).as_ref().map(|g| g.peer == peer).unwrap_or(false) { self.moon_end("they disconnected"); }
                // The other PC ended our connection to it.
                let ours = crate::lock(&self.moon.out).as_ref().map(|o| o.peer == peer).unwrap_or(false);
                if ours { self.moon_disconnect("stopped sharing"); }
                Ok(json!({"ok": true}))
            }
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    /// "Can I connect?" from a device: only while the switch is on and only from a person on the list; asked on this
    /// PC's screen unless they're set to Always allow. A yes starts Sunshine (only this PC can reach it) and lets
    /// that device's tunnel in.
    async fn moon_ask(self: &Arc<Self>, peer: &str) -> Result<Value, String> {
        let me = self.name();
        let sh = load();
        if !sh.on { return Err(format!("{me} isn't letting anyone connect right now")); }
        let person = sh.person(peer).cloned().ok_or_else(|| format!("{me} hasn't added this PC to its family and friends"))?;
        let who = { let n = self.peer_name(peer); if n.is_empty() { person.name.clone() } else { n } };
        if let Some(g) = crate::lock(&self.moon.grant).as_ref() {
            if g.peer != peer { return Err(format!("Someone else is connected to {me} right now. Try again later")); }
        }
        if crate::lock(&self.moon.grant).as_ref().map(|g| g.peer == peer).unwrap_or(false) { self.moon_end("they connected again"); }
        if self.moon.asking.swap(true, Ordering::SeqCst) { return Err(format!("{me} is asking about another connection. Try again in a minute")); }
        let answer = if person.always { ("allow".to_string(), String::new()) } else {
            let (w, wh, l) = (who.clone(), person.what.clone(), person.limit);
            eprintln!("hearth-link: moonlight: {who} wants to connect: asking on this PC");
            tokio::task::spawn_blocking(move || ask_person(&w, &wh, l)).await.unwrap_or(("deny".into(), String::new()))
        };
        self.moon.asking.store(false, Ordering::SeqCst);
        match answer.0.as_str() {
            "always" => {
                let id = peer.to_string();
                let _ = save(|m| { let _ = change_person(m, &id, &json!({"always": true})); });
                eprintln!("hearth-link: moonlight: {who}: allowed, and always from now on (the person here chose that)");
            }
            "allow" => eprintln!("hearth-link: moonlight: {who}: allowed{}", if person.always { " (Always allow)" } else { "" }),
            _ => {
                eprintln!("hearth-link: moonlight: {who}: not allowed{}", if answer.1.is_empty() { String::new() } else { format!(" ({})", answer.1) });
                self.moon_said(format!("Said no to {who}"));
                return Err(if answer.1.is_empty() { format!("{me} said no") } else { format!("{me}: {}", answer.1) });
            }
        }
        // The switch could have been turned off while the question showed.
        if !load().on { return Err(format!("{me} stopped letting anyone connect")); }
        let st = tokio::task::spawn_blocking(|| helper(&["family", "start"], 90)).await.map_err(|e| e.to_string())??;
        let port = st["port"].as_u64().map(|p| p as u16).filter(|p| *p > 1024).unwrap_or(BASE);
        let key = client_key(peer);
        let k2 = key.clone();
        let paired = tokio::task::spawn_blocking(move || helper(&["family", "paired", k2.as_str()], 20)).await.ok().and_then(|r| r.ok()).map(|v| v["paired"] == true).unwrap_or(false);
        let now = crate::now_ms();
        let until = if person.limit > 0 { now + person.limit as u64 * minute_ms() } else { 0 };
        let gen = self.moon.gen.fetch_add(1, Ordering::SeqCst) + 1;
        let notice = { let (w, wh) = (who.clone(), person.what.clone()); tokio::task::spawn_blocking(move || show_notice(&w, &wh, until)).await.ok().flatten() };
        let g = Grant { gen, peer: peer.to_string(), name: who.clone(), what: person.what.clone(), until, started: now, port, conns: Vec::new(), used: false, notice };
        let _ = crate::write_atomic(&runtime_file(), grant_json(&g).to_string().as_bytes());
        *crate::lock(&self.moon.grant) = Some(g);
        if until > 0 {
            let n = self.clone();
            tokio::spawn(async move { tokio::time::sleep(Duration::from_millis(until - now)).await; n.moon_end_if(gen, "the time limit ended the session"); });
        }
        let n = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(UNUSED_GRANT).await;
            let unused = crate::lock(&n.moon.grant).as_ref().map(|g| g.gen == gen && !g.used).unwrap_or(false);
            if unused { n.moon_end("Moonlight never connected"); }
        });
        Ok(json!({"ok": true, "what": person.what, "app": app_for(&person.what), "port": port, "paired": paired, "until": until, "name": me}))
    }

    /// The tunnel from a device this PC said yes to (anything else is closed at once).
    pub async fn moon_tunnel_in(self: Arc<Self>, conn: Connection) {
        let remote = conn.remote_id().to_string();
        let gen_port = {
            let paired = crate::lock(&self.peers).is_paired(&remote);
            let mut g = crate::lock(&self.moon.grant);
            match g.as_mut() {
                Some(g) if paired && g.peer == remote && (g.until == 0 || g.until > crate::now_ms()) => { g.conns.push(conn.clone()); g.used = true; Some((g.gen, g.port)) }
                _ => None,
            }
        };
        let Some((gen, port)) = gen_port else { conn.close(VarInt::from_u32(403), b"not now"); return; };
        if !load().on { conn.close(VarInt::from_u32(403), b"not sharing"); self.moon_end_if(gen, "sharing is off"); return; }
        if let Some(g) = crate::lock(&self.moon.grant).as_mut() { let _ = crate::write_atomic(&runtime_file(), grant_json(g).to_string().as_bytes()); }
        eprintln!("hearth-link: moonlight: {}: connected", self.peer_name(&remote));
        serve_tunnel(conn.clone(), port).await;
        let left = {
            let mut g = crate::lock(&self.moon.grant);
            match g.as_mut() { Some(g) if g.gen == gen => { g.conns.retain(|c| c.close_reason().is_none()); g.conns.len() } _ => 1 }
        };
        if left == 0 { self.moon_end_if(gen, "they disconnected"); }
    }

    /// The other side of a session this PC started: Moonlight is stopped and the tunnel closed.
    pub fn moon_disconnect(&self, why: &str) {
        let o = crate::lock(&self.moon.out).take();
        if let Some(o) = o {
            if let Some(c) = &o.conn { c.close(VarInt::from_u32(0), b"done"); }
            if o.pid > 0 { unsafe { libc::kill(o.pid as i32, libc::SIGTERM); } }
            eprintln!("hearth-link: moonlight: connection to {} ended: {why}", o.name);
            *crate::lock(&self.moon.last) = Some((crate::now_ms(), format!("{}: {why}", o.name)));
        }
    }

    /// Connect to a PC that lets this one: ask it, open the tunnel, start Moonlight in its own network (pairing it
    /// first with a one-time PIN when that PC's Sunshine doesn't know it yet). Answers once Moonlight is starting.
    async fn moon_connect(self: &Arc<Self>, id: &str) -> Result<Value, String> {
        let name = self.peer_name(id);
        if crate::lock(&self.peers).get(id).map(|p| p.moonlight.is_empty()).unwrap_or(true) { return Err(format!("{name} hasn't let this PC connect")); }
        if crate::lock(&self.moon.out).is_some() { return Err("This PC is connected to another one already: disconnect first".into()); }
        let ml = moonlight_cmd().ok_or("Moonlight isn't installed: Settings › Remote play › Moonlight › Install")?;
        drop(ml);
        let g = match tokio::time::timeout(ANSWER_WAIT + Duration::from_secs(60), self.request(id, &json!({"op": "moonlight.ask"}))).await {
            Ok(Ok(v)) => v,
            Ok(Err(e)) if e.contains("Unknown request") => return Err(format!("{name} needs a Hearth OS update for this")),
            Ok(Err(e)) => return Err(e.trim_start_matches("connection: ").to_string()),
            Err(_) => return Err(format!("{name} didn't answer")),
        };
        let port = g["port"].as_u64().map(|p| p as u16).filter(|p| *p > 1024).unwrap_or(BASE);
        let addr = self.addr_of(id, None)?;
        let conn = match tokio::time::timeout(Duration::from_secs(25), self.ep.connect(addr, ALPN)).await {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => return Err(format!("Couldn't reach {name}: {e}")),
            Err(_) => return Err(format!("Couldn't reach {name}")),
        };
        let relay = !conn.paths().iter().any(|p| p.is_selected() && matches!(p.remote_addr(), iroh::TransportAddr::Ip(_)));
        if conn.max_datagram_size().is_none() { conn.close(VarInt::from_u32(0), b"no datagrams"); return Err(format!("The connection to {name} can't carry Moonlight's picture")); }
        let host = host_addr(id);
        let pin = if g["paired"] == true { None } else { Some(format!("{:04}", u16::from_le_bytes(crate::random_bytes::<2>()) % 10000)) };
        let exe = std::env::current_exe().map_err(|e| e.to_string())?.to_string_lossy().to_string();
        let mut prog = vec![exe, "moon-session".into(), host.to_string(), g["app"].as_str().unwrap_or("Desktop").to_string()];
        // Through the relay a lower bitrate keeps it smooth (owner's call how far: docs/HEARTH-LINK.md).
        if relay { prog.push("5000".into()); }
        let mut envs: Vec<(String, String)> = Vec::new();
        if let Some(p) = &pin { envs.push(("HEARTH_MOON_PIN".into(), p.clone())); }
        let pv = match tokio::task::spawn_blocking(move || spawn_private(&prog, &envs, host, port)).await.map_err(|e| e.to_string())? {
            Ok(p) => p,
            Err(e) => { conn.close(VarInt::from_u32(0), b"no moonlight"); let _ = self.request(id, &json!({"op": "moonlight.end"})).await; return Err(e); }
        };
        let Private { mut child, tcp, udp } = pv;
        let mut ls = Vec::new();
        let mut us = Vec::new();
        for l in tcp { let _ = l.set_nonblocking(true); if let Ok(l) = tokio::net::TcpListener::from_std(l) { ls.push(l); } }
        for u in udp { let _ = u.set_nonblocking(true); if let Ok(u) = tokio::net::UdpSocket::from_std(u) { us.push(u); } }
        let gen = self.moon.gen.fetch_add(1, Ordering::SeqCst) + 1;
        *crate::lock(&self.moon.out) = Some(Out { gen, peer: id.to_string(), name: name.clone(), conn: Some(conn.clone()), pid: child.id(), started: crate::now_ms(),
                                                    until: g["until"].as_u64().unwrap_or(0), relay });
        eprintln!("hearth-link: moonlight: connecting to {name}{}", if relay { " through the relay" } else { "" });
        let c2 = conn.clone();
        tokio::spawn(async move { run_tunnel(c2, ls, us).await; });
        if let Some(p) = pin {
            let n = self.clone(); let id2 = id.to_string();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(800)).await;
                if let Err(e) = n.request(&id2, &json!({"op": "moonlight.pin", "pin": p})).await { eprintln!("hearth-link: moonlight: pairing Moonlight: {e}"); }
            });
        }
        // Moonlight ends (closed, or the other PC stopped): everything with it.
        let n = self.clone(); let id2 = id.to_string();
        tokio::spawn(async move {
            let code = tokio::task::spawn_blocking(move || child.wait().ok().and_then(|s| s.code())).await.ok().flatten();
            let ours = crate::lock(&n.moon.out).as_ref().map(|o| o.gen == gen).unwrap_or(false);
            if ours {
                n.moon_disconnect(match code { Some(4) => "Moonlight couldn't pair with it", Some(3) => "Moonlight couldn't start", _ => "Moonlight closed" });
                let _ = tokio::time::timeout(Duration::from_secs(8), n.request(&id2, &json!({"op": "moonlight.end"}))).await;
            }
        });
        let n = self.clone();
        tokio::spawn(async move {
            let _ = conn.closed().await;
            let ours = crate::lock(&n.moon.out).as_ref().map(|o| o.gen == gen).unwrap_or(false);
            if ours { n.moon_disconnect("the connection ended"); }
        });
        Ok(json!({"ok": true, "name": name, "what": g["what"], "relay": relay, "until": g["until"], "pairing": g["paired"] != true}))
    }

    /// The PCs that let this one connect, for the lists in Hearth Games, Game Mode, Firehold and Settings. `check`:
    /// each is asked once now (that's how online is known: nothing asks on its own).
    async fn moon_friends(self: &Arc<Self>, check: bool) -> Value {
        let list: Vec<(String, String, String)> = crate::lock(&self.peers).list.iter().filter(|p| !p.moonlight.is_empty()).map(|p| (p.id.clone(), p.name.clone(), p.moonlight.clone())).collect();
        let mut set = tokio::task::JoinSet::new();
        for (id, name, what) in list {
            let n = self.clone();
            set.spawn(async move {
                let (online, v) = if check {
                    match tokio::time::timeout(Duration::from_secs(6), n.request(&id, &json!({"op": "moonlight.hello"}))).await {
                        Ok(Ok(v)) => (true, v),
                        _ => (false, Value::Null),
                    }
                } else { (crate::lock(&n.peers).get(&id).map(|p| crate::now_ms().saturating_sub(p.last_seen) < 60_000).unwrap_or(false), Value::Null) };
                // It said it took this PC off its list: gone here too.
                if online && v["approved"] == false {
                    let mut ps = crate::lock(&n.peers);
                    if let Some(p) = ps.get_mut(&id) { p.moonlight.clear(); let _ = ps.save(); }
                    return None;
                }
                let what = v["what"].as_str().map(str::to_string).unwrap_or(what);
                Some(json!({"id": id, "name": name, "online": online, "sharing": if online { v["sharing"].clone() } else { Value::Null }, "what": what,
                            "ask": v["ask"], "limitMinutes": v["limitMinutes"]}))
            });
        }
        let mut out = Vec::new();
        while let Some(r) = set.join_next().await { if let Ok(Some(v)) = r { out.push(v); } }
        out.sort_by(|a, b| (b["online"] == true).cmp(&(a["online"] == true)).then_with(|| a["name"].as_str().cmp(&b["name"].as_str())));
        let o = crate::lock(&self.moon.out).as_ref().map(|o| json!({"peer": o.peer, "name": o.name, "started": o.started, "until": o.until, "relay": o.relay}));
        json!({"friends": out, "connected": o, "moonlight": moonlight_cmd().is_some(), "last": self.moon_last()})
    }

    fn moon_last(&self) -> Value {
        crate::lock(&self.moon.last).as_ref().filter(|(t, _)| crate::now_ms().saturating_sub(*t) < 120_000).map(|(t, s)| json!({"at": t, "text": s})).unwrap_or(Value::Null)
    }

    /// This PC's side: the switch, the people, an "Add a person" code, who's connected now.
    fn moon_status(&self) -> Value {
        let sh = load();
        let ps = crate::lock(&self.peers);
        let now = crate::now_ms();
        let people: Vec<Value> = sh.people.iter().filter(|(id, _)| ps.is_paired(id)).map(|(id, p)| {
            let peer = ps.get(id);
            let online = peer.map(|p| now.saturating_sub(p.last_seen) < 60_000).unwrap_or(false) || crate::lock(&self.found).get(id).map(|f| now.saturating_sub(f.at) < 60_000).unwrap_or(false);
            let mut v = person_json(p);
            v["id"] = json!(id);
            if let Some(n) = peer.map(|p| p.name.clone()).filter(|n| !n.is_empty()) { v["name"] = json!(n); }
            v["online"] = json!(online);
            v["relation"] = json!(peer.map(|p| p.rel().as_str()).unwrap_or("friend"));
            v
        }).collect();
        drop(ps);
        let inv = (*crate::lock(&self.moon.invite)).filter(|u| *u > now);
        let invite = if inv.is_some() { crate::pair::status_json(&crate::lock(&self.pairing)) } else { Value::Null };
        let session = crate::lock(&self.moon.grant).as_ref().map(grant_json);
        json!({"on": sh.on, "readable": sh.readable, "people": people, "invite": invite, "session": session, "asking": self.moon.asking.load(Ordering::SeqCst),
               "name": self.name(), "last": self.moon_last()})
    }

    pub async fn moon_command(self: &Arc<Self>, cmd: &str, a: &Value) -> Result<Value, String> {
        let id = || a["id"].as_str().filter(|x| valid_id(x)).map(str::to_string).ok_or_else(|| "Missing 'id'".to_string());
        match cmd {
            "moonlight.share.status" => Ok(self.moon_status()),
            "moonlight.share.set" => {
                let on = a["on"].as_bool().ok_or("on: true or false")?;
                if !load().readable { return Err("Your Family and friends settings couldn't be read, so nothing was changed".into()); }
                if on {
                    // Sunshine set up first (only this PC can reach it): if that fails, the switch stays off.
                    tokio::task::spawn_blocking(|| helper(&["family", "on"], 120)).await.map_err(|e| e.to_string())??;
                    save(|m| { m.insert("on".into(), json!(true)); })?;
                    eprintln!("hearth-link: moonlight: family and friends can connect now (the person here turned it on)");
                    return Ok(self.moon_status());
                }
                save(|m| { m.insert("on".into(), json!(false)); })?;
                *crate::lock(&self.moon.invite) = None;
                self.moon_end("sharing was turned off");
                let note = tokio::task::spawn_blocking(|| helper(&["family", "off"], 120)).await.map_err(|e| e.to_string())?.err().unwrap_or_default();
                eprintln!("hearth-link: moonlight: turned off: nobody can connect");
                let mut v = self.moon_status();
                if !note.is_empty() { v["note"] = json!(note); }
                Ok(v)
            }
            "moonlight.share.person" => {
                let id = id()?;
                let mut err = None;
                save(|m| { if let Err(e) = change_person(m, &id, a) { err = Some(e); } })?;
                if let Some(e) = err { return Err(e); }
                // What they get changed: they hear it the next time they look.
                if let Some(w) = a["what"].as_str() {
                    let n = self.clone(); let (id2, w) = (id.clone(), w.to_string());
                    tokio::spawn(async move { let _ = tokio::time::timeout(Duration::from_secs(10), n.request(&id2, &json!({"op": "moonlight.offer", "what": w}))).await; });
                }
                Ok(self.moon_status())
            }
            "moonlight.share.remove" => {
                let id = id()?;
                save(|m| { if let Some(Value::Object(ps)) = m.get_mut("people") { ps.remove(&id); } })?;
                if crate::lock(&self.moon.grant).as_ref().map(|g| g.peer == id).unwrap_or(false) { self.moon_end("they were removed"); }
                let key = client_key(&id);
                let _ = tokio::task::spawn_blocking(move || helper(&["family", "forget", key.as_str()], 30)).await;
                let n = self.clone(); let id2 = id.clone();
                tokio::spawn(async move { let _ = tokio::time::timeout(Duration::from_secs(10), n.request(&id2, &json!({"op": "moonlight.revoke"}))).await; });
                eprintln!("hearth-link: moonlight: {} can't connect anymore (removed)", self.peer_name(&id));
                Ok(self.moon_status())
            }
            "moonlight.share.invite" => {
                if !load().on { return Err("Turn on Let family and friends connect first".into()); }
                let mut v = self.pair_open(&json!({"as": "friend"})).await?;
                let until = v["until"].as_u64().unwrap_or(crate::now_ms() + crate::pair::CODE_LIFETIME_MS);
                // A short code (and its QR code) for someone who isn't here: if the lookup services can't be
                // reached, the 6 digits still work on the same network.
                match self.pair_open(&json!({"internet": true, "as": "friend"})).await {
                    Ok(w) => v = w,
                    Err(e) => v["netError"] = json!(e),
                }
                let until = v["netUntil"].as_u64().unwrap_or(0).max(until);
                *crate::lock(&self.moon.invite) = Some(until);
                Ok(json!({"invite": v}))
            }
            "moonlight.share.invite.close" => {
                *crate::lock(&self.moon.invite) = None;
                self.pair_close();
                Ok(Value::Null)
            }
            "moonlight.stop" => { self.moon_end("stopped on this PC"); Ok(self.moon_status()) }
            "moonlight.friends" => Ok(self.moon_friends(a["check"] == true).await),
            "moonlight.connect" => { let id = id()?; self.moon_connect(&id).await }
            "moonlight.disconnect" => {
                let peer = crate::lock(&self.moon.out).as_ref().map(|o| o.peer.clone());
                self.moon_disconnect("disconnected here");
                if let Some(p) = peer { let n = self.clone(); tokio::spawn(async move { let _ = tokio::time::timeout(Duration::from_secs(8), n.request(&p, &json!({"op": "moonlight.end"}))).await; }); }
                Ok(Value::Null)
            }
            "moonlight.forget" => {
                let id = id()?;
                let mut ps = crate::lock(&self.peers);
                if let Some(p) = ps.get_mut(&id) { p.moonlight.clear(); ps.save()?; }
                Ok(Value::Null)
            }
            "moonlight.join" => {
                // Add a PC with the code its person shows: paired as a friend's device (a device already paired
                // keeps whose it is); that PC then says it lets this one connect.
                let code = a["code"].as_str().unwrap_or("").to_string();
                let mut args = json!({"code": code, "as": "friend", "keep": true});
                if let Some(i) = a["id"].as_str() { args["id"] = json!(i); }
                else if crate::pair::clean_code(&code).is_some() {
                    // Six digits and no PC picked: the one on this network showing a code right now.
                    let now = crate::now_ms();
                    let showing: Vec<String> = crate::lock(&self.found).values().filter(|f| f.pairing && now.saturating_sub(f.at) < 7_000).map(|f| f.id.clone()).collect();
                    match showing.as_slice() {
                        [one] => args["id"] = json!(one),
                        [] => return Err("No PC on this network is showing a code. Ask them for the code for anywhere (like K7M-4QX-92HC)".into()),
                        _ => return Err("More than one PC here is showing a code: use the code for anywhere instead".into()),
                    }
                }
                self.pair_join(&args).await
            }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(c: char) -> String { std::iter::repeat(c).take(64).collect() }

    #[test]
    fn ports_follow_sunshines_own() {
        assert_eq!(ports(47989), ([47984, 47989, 48010], [47998, 47999, 48000]));
        assert_eq!(ports(50000), ([49995, 50000, 50021], [50009, 50010, 50011]));
    }

    #[test]
    fn a_packet_split_and_put_back_together() {
        let data: Vec<u8> = (0..3000u32).map(|i| (i % 251) as u8).collect();
        let parts = frag(2, 7, &data, 1200).unwrap();
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().all(|p| p.len() <= 1200));
        let mut re = Reasm::default();
        // Out of order, as datagrams can come.
        assert_eq!(re.take(&parts[2]), None);
        assert_eq!(re.take(&parts[0]), None);
        assert_eq!(re.take(&parts[1]), Some((2, data.clone())));
        // Small ones go whole.
        let one = frag(1, 8, b"ping", 1200).unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(re.take(&one[0]), Some((1, b"ping".to_vec())));
        // An empty packet stays a packet.
        assert_eq!(re.take(&frag(0, 9, b"", 1200).unwrap()[0]), Some((0, Vec::new())));
        // Too big even for 8 parts: dropped, as a network would.
        assert!(frag(0, 1, &vec![0u8; 20_000], 1200).is_none());
        // A lost part loses the packet, never a mix with the next one.
        let a = frag(0, 10, &data, 1200).unwrap();
        let b: Vec<u8> = data.iter().map(|x| x ^ 0xff).collect();
        let bp = frag(0, 11, &b, 1200).unwrap();
        assert_eq!(re.take(&a[0]), None);
        assert_eq!(re.take(&bp[0]), None);
        assert_eq!(re.take(&bp[1]), None);
        assert_eq!(re.take(&bp[2]), Some((0, b)));
        // Nonsense is ignored.
        assert_eq!(re.take(&[0, 0]), None);
        assert_eq!(re.take(&[0, 0, 0, 0x55]), None);
    }

    #[test]
    fn the_choices_are_read_safely() {
        let m = json!({"on": true, "people": {
            id('a'): {"name": "Chris's PC\u{7}", "always": true, "what": "games", "limitMinutes": 60},
            id('b'): {"name": "Sam", "what": "everything", "limitMinutes": 7},
            "not-an-id": {"name": "x"}
        }});
        let s = parse(m.as_object().unwrap());
        assert!(s.on);
        assert_eq!(s.people.len(), 2);
        let a = s.person(&id('a')).unwrap();
        assert_eq!((a.name.as_str(), a.always, a.what.as_str(), a.limit), ("Chris's PC", true, "games", 60));
        // Anything unknown reads as the default (asked, whole desktop, no limit).
        let b = s.person(&id('b')).unwrap();
        assert_eq!((b.always, b.what.as_str(), b.limit), (false, "desktop", 0));
        // Off unless it says true.
        assert!(!parse(json!({"on": "yes"}).as_object().unwrap()).on);
        assert!(!Share::default().on);
    }

    #[test]
    fn changing_a_person_is_checked() {
        let mut m = json!({"people": {id('a'): {"name": "Chris", "always": false, "what": "desktop", "limitMinutes": 0}}}).as_object().unwrap().clone();
        change_person(&mut m, &id('a'), &json!({"what": "games", "limitMinutes": 120, "always": true})).unwrap();
        assert_eq!(m["people"][id('a')], json!({"name": "Chris", "always": true, "what": "games", "limitMinutes": 120}));
        assert!(change_person(&mut m, &id('a'), &json!({"what": "files"})).is_err());
        assert!(change_person(&mut m, &id('a'), &json!({"limitMinutes": 45})).is_err());
        assert!(change_person(&mut m, &id('b'), &json!({"always": true})).is_err());
    }

    #[test]
    fn saving_never_writes_over_a_file_it_couldnt_read() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("family-moonlight.json");
        std::env::set_var("HEARTH_FAMILY_MOONLIGHT", &f);
        assert_eq!(load(), Share::default());
        save(|m| { m.insert("on".into(), json!(true)); }).unwrap();
        assert!(load().on);
        // Someone else's key in the same file stays (merged).
        let mut v: Value = serde_json::from_slice(&std::fs::read(&f).unwrap()).unwrap();
        v["other"] = json!(1);
        std::fs::write(&f, v.to_string()).unwrap();
        save(|m| { m.insert("on".into(), json!(false)); }).unwrap();
        let v: Value = serde_json::from_slice(&std::fs::read(&f).unwrap()).unwrap();
        assert_eq!(v["other"], json!(1));
        assert_eq!(v["on"], json!(false));
        std::env::remove_var("HEARTH_FAMILY_MOONLIGHT");
    }

    #[test]
    fn answers_and_names() {
        assert_eq!(answer_word("allow\n"), "allow");
        assert_eq!(answer_word("some log line\nalways\n"), "always");
        assert_eq!(answer_word(""), "deny");
        assert_eq!(answer_word("deny"), "deny");
        assert_eq!(answer_word("ALLOW!"), "deny");
        assert_eq!(app_for("games"), "Steam Big Picture");
        assert_eq!(app_for("desktop"), "Desktop");
        assert_eq!(client_key(&id('c')), "Hearth cccccccc");
        let a = host_addr(&id('a'));
        assert_eq!(a.octets()[..2], [127, 77]);
        assert_ne!(a, host_addr(&id('b')));
        assert!(a.is_loopback());
        assert!(is_op("moonlight.ask") && !is_op("sync"));
    }
}
