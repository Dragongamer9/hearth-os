//! Pairing two devices with a short code.
//!
//! Device A opens pairing (Settings › Hearth Link › Pair a device) and shows a 6-digit code for 5 minutes. Device B
//! picks A from the devices it sees (or types A's id, for a device somewhere else) and enters the code. B connects to
//! A over QUIC (ALPN hearth/link-pair/1; A only takes these while its code is showing) and both run SPAKE2 with the
//! code as the password, bound to both devices' keys as the TLS handshake proved them. Then each proves it got the
//! same key (a BLAKE3 MAC over both keys), and only then do they keep each other as paired.
//!
//! Why not just send the code: someone in the middle (who'd have to be on the connection with their own key) would
//! learn the code from a hash by trying all million codes. With SPAKE2 every wrong guess costs a whole pairing
//! attempt, and A closes pairing after 5 wrong codes. Binding the keys means a key exchanged with someone in the
//! middle can't produce the right confirmation, so nobody but the two devices ends up paired.

use crate::wire::{read_msg, write_msg};
use serde_json::{json, Value};
use spake2::{Ed25519Group, Identity, Password, Spake2};

pub const CODE_LIFETIME_MS: u64 = 5 * 60 * 1000;
pub const MAX_TRIES: u32 = 5;

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

/// The joining side (B), on a connection to the host. Returns the host's name.
pub async fn join(conn: &iroh::endpoint::Connection, me: &[u8; 32], my_name: &str, code: &str) -> Result<String, String> {
    let host_key = *conn.remote_id().as_bytes();
    let (mut s, mut r) = conn.open_bi().await.map_err(|e| e.to_string())?;
    let (state, msg) = start(code, &host_key, me);
    write_msg(&mut s, &json!({"v": 1, "name": my_name, "msg": crate::hex(&msg)})).await?;
    let a = read_msg(&mut r).await?;
    if let Some(e) = a["error"].as_str() { return Err(e.to_string()); }
    let their = a["msg"].as_str().and_then(crate::unhex).ok_or("The other device didn't answer the pairing")?;
    let host_name: String = a["name"].as_str().unwrap_or("Hearth").chars().filter(|c| !c.is_control()).take(40).collect();
    let key = state.finish(&their).map_err(|_| "Pairing failed".to_string())?;
    write_msg(&mut s, &json!({"confirm": crate::hex(&confirm_tag(&key, b"join", &host_key, me))})).await?;
    let c = read_msg(&mut r).await?;
    if let Some(e) = c["error"].as_str() { return Err(e.to_string()); }
    let theirs = c["confirm"].as_str().and_then(crate::unhex).unwrap_or_default();
    if !tag_eq(&theirs, &confirm_tag(&key, b"host", &host_key, me)) { return Err("That code isn't right".into()); }
    let _ = s.finish();
    Ok(host_name)
}

/// The host side (A), for one incoming pairing connection: `code` is the code on screen. Ok(joiner's name) when the
/// joiner proved it has the code; Err(true) for a wrong code (counts as a try), Err(false) for anything else.
pub async fn host(conn: &iroh::endpoint::Connection, me: &[u8; 32], my_name: &str, code: &str) -> Result<String, bool> {
    let joiner = *conn.remote_id().as_bytes();
    let (mut s, mut r) = conn.accept_bi().await.map_err(|_| false)?;
    let b = read_msg(&mut r).await.map_err(|_| false)?;
    let their = b["msg"].as_str().and_then(crate::unhex).ok_or(false)?;
    let name: String = b["name"].as_str().unwrap_or("Hearth").chars().filter(|c| !c.is_control()).take(40).collect();
    let (state, msg) = start(code, me, &joiner);
    write_msg(&mut s, &json!({"v": 1, "name": my_name, "msg": crate::hex(&msg)})).await.map_err(|_| false)?;
    let key = match state.finish(&their) { Ok(k) => k, Err(_) => return Err(true) };
    let c = read_msg(&mut r).await.map_err(|_| false)?;
    let theirs = c["confirm"].as_str().and_then(crate::unhex).unwrap_or_default();
    if !tag_eq(&theirs, &confirm_tag(&key, b"join", me, &joiner)) {
        let _ = write_msg(&mut s, &json!({"error": "That code isn't right"})).await;
        let _ = s.finish();
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        return Err(true);
    }
    write_msg(&mut s, &json!({"confirm": crate::hex(&confirm_tag(&key, b"host", me, &joiner))})).await.map_err(|_| false)?;
    let _ = s.finish();
    // Wait for the joiner to read it (it closes the connection when it has).
    let _ = tokio::time::timeout(std::time::Duration::from_secs(3), conn.closed()).await;
    Ok(name)
}

/// What the joiner sends the host first; used by tests of the message exchange without a network.
pub fn exchange_offline(code_a: &str, code_b: &str) -> bool {
    let (ka, kb) = ([1u8; 32], [2u8; 32]);
    let (sa, ma) = start(code_a, &ka, &kb);
    let (sb, mb) = start(code_b, &ka, &kb);
    let (Ok(a), Ok(b)) = (sa.finish(&mb), sb.finish(&ma)) else { return false };
    tag_eq(&confirm_tag(&a, b"join", &ka, &kb), &confirm_tag(&b, b"join", &ka, &kb))
}

pub fn status_json(code: &Option<(String, u64, u32)>) -> Value {
    match code {
        Some((c, until, tries)) if *until > crate::now_ms() => json!({"code": c, "until": until, "triesLeft": MAX_TRIES.saturating_sub(*tries)}),
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
    fn codes() {
        for _ in 0..50 { let c = new_code(); assert_eq!(c.len(), 6); assert!(clean_code(&c).is_some()); }
        assert_eq!(clean_code(" 123-456 "), Some("123456".into()));
        assert_eq!(clean_code("12345"), None);
        assert_eq!(clean_code("12345a"), None);
    }
}
