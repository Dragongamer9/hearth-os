//! Finding Hearth devices on the same network: every device says who it is (its key, name, QUIC port, and whether
//! it's showing a pairing code) by UDP broadcast on port 47741, every few seconds while pairing is open and every
//! 20 seconds otherwise; announcements are only taken from local addresses. A paired device heard this way is
//! connected to directly at that address (no relay, no internet needed). An announcement only says a device is
//! there: connecting still proves its key (QUIC's TLS), so a false one can't stand in for a paired device.

use serde_json::{json, Value};
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Debug, Clone, PartialEq)]
pub struct Announce { pub id: String, pub name: String, pub port: u16, pub pairing: bool }

pub fn encode(a: &Announce) -> Vec<u8> {
    json!({"hearthLink": 1, "id": a.id, "name": a.name, "port": a.port, "pairing": a.pairing}).to_string().into_bytes()
}

pub fn decode(b: &[u8]) -> Option<Announce> {
    if b.len() > 2048 { return None; }
    let v: Value = serde_json::from_slice(b).ok()?;
    if v["hearthLink"] != 1 { return None; }
    let id = v["id"].as_str().filter(|s| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()))?.to_string();
    let name: String = v["name"].as_str().unwrap_or("Hearth").chars().filter(|c| !c.is_control()).take(40).collect();
    let port = v["port"].as_u64().filter(|p| *p > 0 && *p < 65536)? as u16;
    Some(Announce { id, name, port, pairing: v["pairing"] == true })
}

/// Private, link-local and loopback addresses: the only ones announcements are taken from.
pub fn is_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => v.is_private() || v.is_link_local() || v.is_loopback(),
        IpAddr::V6(v) => v.to_ipv4_mapped().map(|m| is_local(IpAddr::V4(m))).unwrap_or(false) || v.is_loopback()
            || (v.segments()[0] & 0xffc0) == 0xfe80 || (v.segments()[0] & 0xfe00) == 0xfc00,
    }
}

/// The broadcast socket (SO_BROADCAST, the port shared in case two Hearth users are signed in on one PC).
pub fn socket(port: u16) -> std::io::Result<tokio::net::UdpSocket> {
    let s = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    s.set_reuse_address(true)?;
    let _ = s.set_reuse_port(true);
    s.set_broadcast(true)?;
    s.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)).into())?;
    s.set_nonblocking(true)?;
    tokio::net::UdpSocket::from_std(s.into())
}

/// Every local network's broadcast address (and the general one).
pub fn targets() -> Vec<Ipv4Addr> {
    let mut out = vec![Ipv4Addr::BROADCAST];
    for i in if_addrs::get_if_addrs().unwrap_or_default() {
        if i.is_loopback() { continue; }
        if let if_addrs::IfAddr::V4(a) = i.addr {
            if !is_local(IpAddr::V4(a.ip)) { continue; }
            let b = a.broadcast.unwrap_or_else(|| Ipv4Addr::from(u32::from(a.ip) | !u32::from(a.netmask)));
            if !out.contains(&b) { out.push(b); }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn announcements_both_ways_and_checked() {
        let a = Announce { id: "ab".repeat(32), name: "Deck".into(), port: 47740, pairing: true };
        assert_eq!(decode(&encode(&a)), Some(a.clone()));
        assert_eq!(decode(br#"{"hearthLink":1,"id":"zz","port":1}"#), None);
        assert_eq!(decode(br#"{"id":"abab","port":1}"#), None);
        assert_eq!(decode(b"not json"), None);
    }
    #[test]
    fn local_addresses_only() {
        assert!(is_local("192.168.1.4".parse().unwrap()));
        assert!(is_local("10.9.0.1".parse().unwrap()));
        assert!(is_local("fe80::1".parse().unwrap()));
        assert!(!is_local("8.8.8.8".parse().unwrap()));
        assert!(!is_local("2001:db8::1".parse().unwrap()));
    }
}
