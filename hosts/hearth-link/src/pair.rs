//! Pairing two devices with a short code.
//!
//! On the same network: device A opens pairing (Settings › Hearth Link › Pair a device), says so in its network
//! announcements (discovery.rs) and shows a 6-digit code for 5 minutes. Device B lists the Hearth devices around it
//! that are showing a code; the person picks A and types the code. Somewhere else: A also shows a 10-character code
//! for 10 minutes (shortcode.rs) and B finds A from that code alone (lookup.rs); nothing long to type.
//!
//! Either way B connects to A over QUIC (ALPN hearth/link-pair/1; A only takes these while a code is showing, one at
//! a time) and both run SPAKE2 with the code as the password, bound to both devices' keys as the TLS handshake
//! proved them. Then each proves it got the same key (a BLAKE3 MAC over both keys), and only then do they keep each
//! other as paired.
//!
//! Why not just send the code: someone in the middle (who'd have to be on the connection with their own key) would
//! learn the code from a hash by trying all codes. With SPAKE2 every wrong guess costs a whole pairing attempt, and
//! A stops pairing after 5 wrong codes (both codes count together). A wrong code doesn't end anything on B's side:
//! B is told how many tries are left and can type it again. Binding the keys means a key exchanged with someone in
//! the middle can't produce the right confirmation, so nobody but the two devices ends up paired.

use crate::wire::{read_msg, write_msg};
use serde_json::{json, Value};
use spake2::{Ed25519Group, Identity, Password, Spake2};

pub const CODE_LIFETIME_MS: u64 = 5 * 60 * 1000;
pub const NET_CODE_LIFETIME_MS: u64 = 10 * 60 * 1000;
pub const MAX_TRIES: u32 = 5;

/// Pairing open on this device: the 6-digit code, and the short code for somewhere else once asked for.
#[derive(Debug, Clone)]
pub struct Pairing { pub code: String, pub until: u64, pub tries: u32, pub net: Option<(String, u64)> }

impl Pairing {
    pub fn new(lifetime_ms: u64) -> Self { Pairing { code: new_code(), until: crate::now_ms() + lifetime_ms, tries: 0, net: None } }
    /// Still taking attempts: a code showing and tries left.
    pub fn open(&self) -> bool { self.tries < MAX_TRIES && (self.until > crate::now_ms() || self.net_code().is_some()) }
    pub fn lan_code(&self) -> Option<&str> { (self.until > crate::now_ms()).then_some(self.code.as_str()) }
    pub fn net_code(&self) -> Option<&str> { self.net.as_ref().filter(|(_, u)| *u > crate::now_ms()).map(|(c, _)| c.as_str()) }
    /// The code an attempt is checked against: `with` is what the joiner says it typed ("net": the short code).
    pub fn code_for(&self, with: &str) -> Result<String, &'static str> {
        if with == "net" { self.net_code().map(str::to_string).ok_or("expired") } else { self.lan_code().map(str::to_string).ok_or("expired") }
    }
}

/// A new 6-digit code.
pub fn new_code() -> String {
    let n = u32::from_le_bytes(crate::random_bytes::<4>()) % 1_000_000;
    format!("{n:06}")
}

/// Only digits, and 6 of them (spaces and dashes people type are ignored).
pub fn clean_code(s: &str) -> Option<String> {
    let d: String = s.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
    (d.len() == 6 && d.chars().all(|c| c.is_ascii_digit())).then_some(d)
}

/// Both devices' keys, the host's (A) first: SPAKE2's identity, so the exchange is tied to these two keys.
pub fn binding(host: &[u8; 32], joiner: &[u8; 32]) -> Vec<u8> {
    let mut v = b"hearth-link pairing v1|".to_vec();
    v.extend_from_slice(host);
    v.extend_from_slice(joiner);
    v
}

/// The proof one side got the same key: `role` is b"host" or b"join".
pub fn confirm_tag(shared: &[u8], role: &[u8], host: &[u8; 32], joiner: &[u8; 32]) -> [u8; 32] {
    let k = blake3::derive_key("hearth-link pairing confirm v1", shared);
    let mut h = blake3::Hasher::new_keyed(&k);
    h.update(role);
    h.update(host);
    h.update(joiner);
    *h.finalize().as_bytes()
}

pub fn start(code: &str, host: &[u8; 32], joiner: &[u8; 32]) -> (Spake2<Ed25519Group>, Vec<u8>) {
    Spake2::<Ed25519Group>::start_symmetric(&Password::new(code.as_bytes()), &Identity::new(&binding(host, joiner)))
}

fn tag_eq(a: &[u8], b: &[u8]) -> bool {
    // Constant time.
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn relay_of(v: &Value) -> Option<String> { v["relay"].as_str().filter(|r| crate::state::valid_relay_url(r)).map(str::to_string) }

/// The words for a wrong code, with the tries the other device has left.
pub fn wrong_code_text(left: u32, host: &str) -> String {
    match left {
        0 => format!("Wrong code. That was the last try: pairing stopped on {host}. Select Pair a device on it again for a new code"),
        1 => "Wrong code. 1 try left".into(),
        n => format!("Wrong code. {n} tries left"),
    }
}

/// The joining side (B), on a connection to the host. `with`: "lan" (the 6-digit code) or "net" (the short code).
/// Returns the host's name and relay (where it can be reached from elsewhere).
pub async fn join(conn: &iroh::endpoint::Connection, me: &[u8; 32], my_name: &str, my_relay: Option<String>, code: &str, with: &str) -> Result<(String, Option<String>), String> {
    let host_key = *conn.remote_id().as_bytes();
    let (mut s, mut r) = conn.open_bi().await.map_err(|e| e.to_string())?;
    let (state, msg) = start(code, &host_key, me);
    write_msg(&mut s, &json!({"v": 2, "name": my_name, "kind": crate::discovery::device_kind(), "relay": my_relay, "with": with, "msg": crate::hex(&msg)})).await?;
    let a = read_msg(&mut r).await?;
    if let Some(e) = a["error"].as_str() { return Err(e.to_string()); }
    let their = a["msg"].as_str().and_then(crate::unhex).ok_or("The other device didn't answer the pairing")?;
    let host_name: String = a["name"].as_str().unwrap_or("Hearth").chars().filter(|c| !c.is_control()).take(40).collect();
    let key = state.finish(&their).map_err(|_| "Pairing failed".to_string())?;
    write_msg(&mut s, &json!({"confirm": crate::hex(&confirm_tag(&key, b"join", &host_key, me))})).await?;
    let c = read_msg(&mut r).await?;
    if c["wrong"] == true { return Err(wrong_code_text(c["triesLeft"].as_u64().unwrap_or(0) as u32, &host_name)); }
    if let Some(e) = c["error"].as_str() { return Err(e.to_string()); }
    let theirs = c["confirm"].as_str().and_then(crate::unhex).unwrap_or_default();
    if !tag_eq(&theirs, &confirm_tag(&key, b"host", &host_key, me)) { return Err("Wrong code".into()); }
    let _ = s.finish();
    Ok((host_name, relay_of(&a)))
}

/// Who's pairing, as the joiner said in its first message.
#[derive(Debug, Clone, Default)]
pub struct Joiner { pub name: String, pub kind: String, pub relay: Option<String> }

/// Why a pairing attempt on the host didn't pair.
#[derive(Debug, PartialEq)]
pub enum HostFail { Wrong, Expired, Other }

/// The host side (A), for one incoming pairing connection. `pairing` is what's showing, `left_if_wrong` the tries
/// that will be left if this one is wrong (told to the joiner), `seen` hears who it is as soon as it says.
pub async fn host(conn: &iroh::endpoint::Connection, me: &[u8; 32], my_name: &str, my_relay: Option<String>, pairing: &Pairing, left_if_wrong: u32,
                  seen: impl Fn(&Joiner)) -> Result<Joiner, (HostFail, Joiner)> {
    let joiner = *conn.remote_id().as_bytes();
    let (mut s, mut r) = conn.accept_bi().await.map_err(|_| (HostFail::Other, Joiner::default()))?;
    let b = read_msg(&mut r).await.map_err(|_| (HostFail::Other, Joiner::default()))?;
    let who = Joiner {
        name: b["name"].as_str().unwrap_or("Hearth").chars().filter(|c| !c.is_control()).take(40).collect(),
        kind: b["kind"].as_str().filter(|k| crate::discovery::KINDS.contains(k)).unwrap_or("").to_string(),
        relay: relay_of(&b),
    };
    seen(&who);
    let fail = |f: HostFail| (f, who.clone());
    let their = b["msg"].as_str().and_then(crate::unhex).ok_or_else(|| fail(HostFail::Other))?;
    // Devices from before short codes (0.6.5) don't say which code they typed: the 6-digit one.
    let code = match pairing.code_for(b["with"].as_str().unwrap_or("lan")) {
        Ok(c) => c,
        Err(_) => {
            let _ = write_msg(&mut s, &json!({"error": "That code expired. Get a new one on the other device"})).await;
            let _ = s.finish();
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            return Err(fail(HostFail::Expired));
        }
    };
    let (state, msg) = start(&code, me, &joiner);
    write_msg(&mut s, &json!({"v": 2, "name": my_name, "relay": my_relay, "msg": crate::hex(&msg)})).await.map_err(|_| fail(HostFail::Other))?;
    let key = match state.finish(&their) { Ok(k) => k, Err(_) => return Err(fail(HostFail::Wrong)) };
    let c = read_msg(&mut r).await.map_err(|_| fail(HostFail::Other))?;
    let theirs = c["confirm"].as_str().and_then(crate::unhex).unwrap_or_default();
    if !tag_eq(&theirs, &confirm_tag(&key, b"join", me, &joiner)) {
        let _ = write_msg(&mut s, &json!({"error": "Wrong code", "wrong": true, "triesLeft": left_if_wrong})).await;
        let _ = s.finish();
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        return Err(fail(HostFail::Wrong));
    }
    write_msg(&mut s, &json!({"confirm": crate::hex(&confirm_tag(&key, b"host", me, &joiner))})).await.map_err(|_| fail(HostFail::Other))?;
    let _ = s.finish();
    // Wait for the joiner to read it (it closes the connection when it has).
    let _ = tokio::time::timeout(std::time::Duration::from_secs(3), conn.closed()).await;
    Ok(who)
}

/// What the joiner sends the host first; used by tests of the message exchange without a network.
pub fn exchange_offline(code_a: &str, code_b: &str) -> bool {
    let (ka, kb) = ([1u8; 32], [2u8; 32]);
    let (sa, ma) = start(code_a, &ka, &kb);
    let (sb, mb) = start(code_b, &ka, &kb);
    let (Ok(a), Ok(b)) = (sa.finish(&mb), sb.finish(&ma)) else { return false };
    tag_eq(&confirm_tag(&a, b"join", &ka, &kb), &confirm_tag(&b, b"join", &ka, &kb))
}

/// The pairing part of the status (for this device's own screen: the codes are shown there, never logged).
pub fn status_json(p: &Option<Pairing>) -> Value {
    match p {
        Some(p) if p.open() => json!({
            "code": p.lan_code(), "until": p.until, "triesLeft": MAX_TRIES.saturating_sub(p.tries),
            "netCode": p.net_code().map(crate::shortcode::format), "netUntil": p.net.as_ref().map(|n| n.1),
        }),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_code_pairs_another_doesnt() {
        assert!(exchange_offline("123456", "123456"));
        assert!(!exchange_offline("123456", "123457"));
    }
    #[test]
    fn keys_are_bound() {
        // The same code with a different key on one side (someone in the middle) gives different keys.
        let code = "424242";
        let (sa, ma) = start(code, &[1; 32], &[2; 32]);
        let (sb, mb) = start(code, &[1; 32], &[3; 32]);
        let a = sa.finish(&mb).unwrap();
        let b = sb.finish(&ma).unwrap();
        assert_ne!(confirm_tag(&a, b"join", &[1; 32], &[2; 32]), confirm_tag(&b, b"join", &[1; 32], &[3; 32]));
    }
    #[test]
    fn pairing_codes_and_expiry() {
        let mut p = Pairing::new(60_000);
        assert!(p.open());
        assert_eq!(p.code_for("lan").as_deref(), Ok(p.code.as_str()));
        assert_eq!(p.code_for("net"), Err("expired"));
        p.net = Some(("23456789A9".into(), crate::now_ms() + 60_000));
        assert_eq!(p.code_for("net").as_deref(), Ok("23456789A9"));
        assert_eq!(status_json(&Some(p.clone()))["netCode"], "234-567-89A9");
        // The 6-digit code ran out: the short code still works.
        p.until = crate::now_ms() - 1;
        assert!(p.open());
        assert_eq!(p.code_for("lan"), Err("expired"));
        assert!(status_json(&Some(p.clone()))["code"].is_null());
        p.net = None;
        assert!(!p.open());
        assert!(status_json(&Some(p.clone())).is_null());
        let mut q = Pairing::new(60_000);
        q.tries = MAX_TRIES;
        assert!(!q.open());
        assert!(wrong_code_text(3, "Deck").contains("3 tries left"));
        assert!(wrong_code_text(0, "Deck").contains("stopped on Deck"));
    }
    #[test]
    fn codes() {
        for _ in 0..50 { let c = new_code(); assert_eq!(c.len(), 6); assert!(clean_code(&c).is_some()); }
        assert_eq!(clean_code(" 123-456 "), Some("123456".into()));
        assert_eq!(clean_code("12345"), None);
        assert_eq!(clean_code("12345a"), None);
    }
}
