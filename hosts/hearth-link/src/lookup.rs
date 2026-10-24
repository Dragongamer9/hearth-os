//! Finding a device from its short code (shortcode.rs), with no Hearth server: the device showing the code publishes
//! a signed pkarr record on public pkarr relays, under a key derived from the code; the device the code is typed on
//! derives the same key and asks the relays for it.
//!
//! Relays: n0's (dns.iroh.link, which iroh uses for its own address lookups) and pkarr's own (relay.pkarr.org, which
//! also puts records on the Mainline DHT). $HEARTH_LINK_PKARR (comma-separated URLs, or "off") replaces them (tests
//! run their own). A record is published if one relay takes it; looking up asks them all and takes the first answer.
//!
//! The record is one TXT name, `_hearth`, whose values are `<n>:<hex>` pieces of the sealed JSON
//! {"v":1,"id","name","kind","relay","addrs"} (or {"v":1,"gone":true} once the code is used, stopped or expired).

use iroh::address_lookup::PkarrRelayClient;
use serde_json::{json, Value};
use std::time::Duration;

pub const DEFAULT_RELAYS: [&str; 2] = ["https://dns.iroh.link/pkarr", "https://relay.pkarr.org"];
const TXT_NAME: &str = "_hearth";
const PIECE: usize = 200;

pub fn relays() -> Vec<url::Url> {
    let list: Vec<String> = match std::env::var("HEARTH_LINK_PKARR") {
        Ok(v) if v.trim() == "off" => Vec::new(),
        Ok(v) if !v.trim().is_empty() => v.split(',').map(|s| s.trim().to_string()).collect(),
        _ => DEFAULT_RELAYS.iter().map(|s| s.to_string()).collect(),
    };
    list.iter().filter_map(|u| u.parse().ok()).collect()
}

fn client(ep: &iroh::Endpoint, url: url::Url) -> Result<PkarrRelayClient, String> {
    let dns = ep.dns_resolver().map_err(|e| e.to_string())?.clone();
    Ok(PkarrRelayClient::new(url, ep.tls_config().clone(), dns))
}

/// The record's TXT values for this content: sealed, in pieces small enough for DNS.
pub fn pieces(seal_key: &[u8; 32], content: &Value) -> Vec<String> {
    let sealed = crate::hex(&crate::shortcode::seal(seal_key, content.to_string().as_bytes()));
    sealed.as_bytes().chunks(PIECE).enumerate().map(|(i, c)| format!("{i}:{}", String::from_utf8_lossy(c))).collect()
}

/// The content back from the TXT values (None: not ours, or sealed with another code).
pub fn unpieces(seal_key: &[u8; 32], values: &[String]) -> Option<Value> {
    let mut parts: Vec<(usize, &str)> = values.iter().filter_map(|v| v.split_once(':').and_then(|(i, h)| Some((i.parse().ok()?, h)))).collect();
    parts.sort_by_key(|p| p.0);
    let hex: String = parts.iter().map(|p| p.1).collect();
    let plain = crate::shortcode::open(seal_key, &crate::unhex(&hex)?)?;
    let v: Value = serde_json::from_slice(&plain).ok()?;
    (v["v"] == 1).then_some(v)
}

/// What another device needs to find this one, kept under the DNS packet's 1000 bytes.
pub fn content(id: &str, name: &str, kind: &str, relay: Option<String>, addrs: Vec<String>) -> Value {
    let mut addrs = addrs;
    addrs.truncate(4);
    let mut v = json!({"v": 1, "id": id, "name": name, "kind": kind, "relay": relay, "addrs": addrs});
    while v.to_string().len() > 360 && !v["addrs"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
        v["addrs"].as_array_mut().unwrap().pop();
    }
    v
}

/// Publishes the record for `code` on every relay; Ok(how many took it), Err when none did.
pub async fn publish(ep: &iroh::Endpoint, code: &str, content: &Value) -> Result<usize, String> {
    let urls = relays();
    if urls.is_empty() { return Err("Short codes are off on this device".into()); }
    let (sk, seal) = { let c = code.to_string(); tokio::task::spawn_blocking(move || crate::shortcode::keys(&c)).await.map_err(|e| e.to_string())? };
    let packet = iroh_dns::pkarr::SignedPacket::from_txt_strings(&sk, TXT_NAME, pieces(&seal, content), 60).map_err(|e| e.to_string())?;
    let mut tasks = Vec::new();
    for u in urls {
        let c = client(ep, u.clone())?;
        let p = packet.clone();
        tasks.push(tokio::spawn(async move { (u.host_str().unwrap_or("?").to_string(), tokio::time::timeout(Duration::from_secs(12), c.publish(&p)).await) }));
    }
    let (mut ok, mut why) = (0, Vec::new());
    for t in tasks {
        match t.await {
            Ok((_, Ok(Ok(())))) => ok += 1,
            Ok((h, Ok(Err(e)))) => why.push(format!("{h}: {e}")),
            Ok((h, Err(_))) => why.push(format!("{h}: no answer")),
            Err(e) => why.push(e.to_string()),
        }
    }
    if ok == 0 { return Err(why.join("; ")); }
    Ok(ok)
}

/// Looks the code up: Ok(the record's content), or why not, in words for the screen.
pub async fn resolve(ep: &iroh::Endpoint, code: &str) -> Result<Value, String> {
    let urls = relays();
    if urls.is_empty() { return Err("Short codes are off on this device".into()); }
    let (sk, seal) = { let c = code.to_string(); tokio::task::spawn_blocking(move || crate::shortcode::keys(&c)).await.map_err(|e| e.to_string())? };
    let key = sk.public();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<Value, String>>(urls.len());
    let n = urls.len();
    for u in urls {
        let c = client(ep, u)?;
        let tx = tx.clone();
        tokio::spawn(async move {
            let r = match tokio::time::timeout(Duration::from_secs(12), c.resolve(key)).await {
                Ok(Ok(p)) => {
                    let vals = p.txt_records(TXT_NAME);
                    unpieces(&seal, &vals).ok_or_else(|| "unreadable".to_string())
                }
                // A relay that answers without the record says "Error resolving http request" (HttpRequest {status});
                // one that can't be reached, "Error sending http request".
                Ok(Err(e)) => Err(format!("{e} {e:?}")),
                Err(_) => Err("no answer".to_string()),
            };
            let _ = tx.send(r).await;
        });
    }
    drop(tx);
    let (mut unreachable, mut missing) = (0, 0);
    for _ in 0..n {
        match rx.recv().await {
            Some(Ok(v)) => return Ok(v),
            Some(Err(e)) => { if e.contains("404") || e.contains("resolving http request") || e.contains("HttpRequest") || e.contains("verify") || e.contains("unreadable") { missing += 1 } else { unreachable += 1 } }
            None => break,
        }
    }
    if missing == 0 && unreachable > 0 { return Err("Couldn't reach the internet to look the code up. Check this device's connection".into()); }
    Err("No device is showing that code. Check it, or get a new one on the other device (codes last 10 minutes)".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pieces_both_ways() {
        let k = [7u8; 32];
        let c = content(&"ab".repeat(32), "Nvidia-PC", "desktop", Some("https://use1-1.relay.n0.iroh.link./".into()), vec!["192.168.1.20:47740".into(), "10.0.0.5:47740".into()]);
        let p = pieces(&k, &c);
        assert!(p.len() > 1 && p.iter().all(|x| x.len() <= PIECE + 3));
        let mut shuffled = p.clone();
        shuffled.reverse();
        assert_eq!(unpieces(&k, &shuffled), Some(c.clone()));
        assert_eq!(unpieces(&[8u8; 32], &p), None);
        // Fits a DNS packet even with a long name and many addresses.
        let big = content(&"ab".repeat(32), &"N".repeat(40), "desktop", Some(format!("https://{}.example.com/", "r".repeat(120))), (0..9).map(|i| format!("[fe80::{i}:1234:5678:9abc]:47740")).collect());
        assert!(big.to_string().len() <= 360 || big["addrs"].as_array().unwrap().is_empty());
        let sk = iroh::SecretKey::from_bytes(&[3u8; 32]);
        assert!(iroh_dns::pkarr::SignedPacket::from_txt_strings(&sk, TXT_NAME, pieces(&k, &big), 60).is_ok());
    }
    #[test]
    fn relay_list() {
        // Set by the e2e tests for their own relay; the defaults otherwise.
        if std::env::var_os("HEARTH_LINK_PKARR").is_none() { assert_eq!(relays().len(), 2); }
    }
}
