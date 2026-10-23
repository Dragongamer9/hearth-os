//! Finding devices, as LocalSend does:
//! - multicast UDP: announce ourselves to 224.0.0.167:53317 on every local network; devices that hear it register
//!   with our HTTPS server. Their announcements are answered by registering with theirs (POST /register), or, if
//!   that fails, with a multicast answer (announce: false) for older apps.
//! - the HTTP scan LocalSend falls back to where multicast doesn't get through (some Wi-Fi routers): POST /register
//!   to every address of our /24 networks, when looking again found nothing.
//! - by address: the window's "Add by address" registers with one IP.

use crate::proto::Info;
use crate::service::{lock, Service};
use socket2::{Domain, Protocol, SockRef, Socket, Type};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;

pub const GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 167);

/// This PC's local IPv4 networks: (address, netmask).
pub fn networks() -> Vec<(Ipv4Addr, Ipv4Addr)> {
    if_addrs::get_if_addrs().unwrap_or_default().into_iter()
        .filter(|i| !i.is_loopback())
        .filter_map(|i| match i.addr { if_addrs::IfAddr::V4(a) if crate::is_local_ip(IpAddr::V4(a.ip)) => Some((a.ip, a.netmask)), _ => None })
        .collect()
}

fn socket(port: u16) -> std::io::Result<tokio::net::UdpSocket> {
    let s = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    s.set_reuse_address(true)?;
    #[cfg(unix)]
    s.set_reuse_port(true)?;
    s.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)).into())?;
    let mut joined = 0;
    for (ip, _) in networks() { if s.join_multicast_v4(&GROUP, &ip).is_ok() { joined += 1; } }
    if joined == 0 { s.join_multicast_v4(&GROUP, &Ipv4Addr::UNSPECIFIED)?; }
    s.set_multicast_loop_v4(true)?;
    s.set_multicast_ttl_v4(1)?;
    s.set_nonblocking(true)?;
    tokio::net::UdpSocket::from_std(s.into())
}

/// Sends `info` to the group on every local network (or the default one).
async fn multicast(udp: &tokio::net::UdpSocket, info: &Info) {
    let Ok(data) = serde_json::to_vec(info) else { return };
    let to = SocketAddr::from((GROUP, crate::DEFAULT_PORT));
    let nets = networks();
    if nets.is_empty() { let _ = udp.send_to(&data, to).await; return; }
    for (ip, _) in nets {
        if SockRef::from(udp).set_multicast_if_v4(&ip).is_ok() { let _ = udp.send_to(&data, to).await; }
    }
}

pub async fn run(svc: Arc<Service>, mut stop: watch::Receiver<bool>) {
    let udp = match socket(crate::DEFAULT_PORT) {
        Ok(u) => Arc::new(u),
        Err(e) => {
            (svc.emit)("discovery-error", serde_json::json!({"error": format!("Can't listen for other devices: {e}")}));
            return;
        }
    };
    // Announce now and a little later (LocalSend does the same: packets get lost on Wi-Fi), then every minute.
    {
        let (svc, udp) = (svc.clone(), udp.clone());
        let mut stop = stop.clone();
        tokio::spawn(async move {
            let mut n = 0u32;
            loop {
                multicast(&udp, &svc.info(Some(true))).await;
                let wait = match n { 0 => 1, 1 => 3, _ => 60 };
                n += 1;
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(wait)) => {}
                    _ = svc.refresh.notified() => {
                        n = 1;
                        let s = svc.clone();
                        tokio::spawn(async move { tokio::time::sleep(Duration::from_secs(3)).await; if lock(&s.devices).is_empty() { scan(s).await; } });
                    }
                    _ = stop.changed() => return,
                }
            }
        });
    }
    let mut buf = vec![0u8; 65536];
    loop {
        let (n, from) = tokio::select! {
            r = udp.recv_from(&mut buf) => match r { Ok(x) => x, Err(_) => { tokio::time::sleep(Duration::from_millis(200)).await; continue; } },
            _ = stop.changed() => return,
        };
        let ip = crate::canonical(from.ip());
        if !crate::is_local_ip(ip) { continue; }
        let Ok(info) = serde_json::from_slice::<Info>(&buf[..n]) else { continue };
        if info.fingerprint.is_empty() || info.fingerprint.eq_ignore_ascii_case(&svc.id.fingerprint) { continue; }
        svc.saw(&info, ip);
        if !info.is_announcement() { continue; }
        // Answer each device at most every few seconds.
        {
            let mut a = lock(&svc.answered);
            if a.get(&info.fingerprint).map(|t| t.elapsed() < Duration::from_secs(3)).unwrap_or(false) { continue; }
            a.insert(info.fingerprint.clone(), Instant::now());
        }
        let (svc2, udp2) = (svc.clone(), udp.clone());
        tokio::spawn(async move {
            let port = info.port.unwrap_or(crate::DEFAULT_PORT);
            if register(&svc2, ip, port, info.https(), Some(info.fingerprint.clone())).await.is_err() {
                multicast(&udp2, &svc2.info(Some(false))).await;
            }
        });
    }
}

/// An HTTPS client for talking to one device: our certificate shown, theirs pinned when we know its fingerprint.
pub fn client(svc: &Service, expect: Option<String>, connect_timeout: Duration) -> Result<reqwest::Client, String> {
    let cfg = crate::tls::client_config(&svc.id, expect)?;
    reqwest::Client::builder().use_preconfigured_tls(cfg).no_proxy().redirect(reqwest::redirect::Policy::none())
        .connect_timeout(connect_timeout).http1_only().build().map_err(|e| e.to_string())
}

pub fn base_url(ip: IpAddr, port: u16, https: bool) -> String {
    let host = match ip { IpAddr::V6(v6) => format!("[{v6}]"), v4 => v4.to_string() };
    format!("{}://{host}:{port}", if https { "https" } else { "http" })
}

/// POST /register to a device: it learns about us, and its answer tells us about it.
pub async fn register(svc: &Arc<Service>, ip: IpAddr, port: u16, https: bool, expect: Option<String>) -> Result<Info, String> {
    let c = client(svc, if https { expect } else { None }, Duration::from_secs(2))?;
    let r = c.post(format!("{}/api/localsend/v2/register", base_url(ip, port, https))).timeout(Duration::from_secs(4))
        .json(&svc.info(None)).send().await.map_err(|e| e.to_string())?;
    if !r.status().is_success() { return Err(format!("{} answered {}", ip, r.status())); }
    let mut info: Info = r.json().await.map_err(|e| e.to_string())?;
    if info.port.is_none() { info.port = Some(port); }
    if info.protocol.is_none() { info.protocol = Some(if https { "https" } else { "http" }.into()); }
    svc.saw(&info, ip);
    Ok(info)
}

/// The window's "Add by address": a device at this IP (and port, default 53317).
pub async fn probe(svc: &Arc<Service>, addr: &str) -> Result<Info, String> {
    let addr = addr.trim();
    let (ip, port) = match addr.parse::<SocketAddr>() {
        Ok(sa) => (sa.ip(), sa.port()),
        Err(_) => (addr.parse::<IpAddr>().map_err(|_| format!("\"{addr}\" isn't an IP address"))?, crate::DEFAULT_PORT),
    };
    if !crate::is_local_ip(ip) { return Err("Only addresses on your local network can be added".into()); }
    match register(svc, ip, port, true, None).await {
        Ok(i) => Ok(i),
        Err(e) => register(svc, ip, port, false, None).await.map_err(|_| format!("Nothing answered at {addr}: {e}")),
    }
}

/// LocalSend's fallback: /register to every address of our /24 networks, 64 at a time.
pub async fn scan(svc: Arc<Service>) {
    let mut targets = vec![];
    for (ip, _) in networks() {
        let o = ip.octets();
        for h in 1..=254u8 {
            if h != o[3] { targets.push(IpAddr::V4(Ipv4Addr::new(o[0], o[1], o[2], h))); }
        }
    }
    let sem = Arc::new(tokio::sync::Semaphore::new(64));
    let mut tasks = vec![];
    for ip in targets {
        let (svc, sem) = (svc.clone(), sem.clone());
        tasks.push(tokio::spawn(async move {
            let Ok(_permit) = sem.acquire().await else { return };
            let _ = tokio::time::timeout(Duration::from_secs(3), register(&svc, ip, crate::DEFAULT_PORT, true, None)).await;
        }));
    }
    for t in tasks { let _ = t.await; }
}
