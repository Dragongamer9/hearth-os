//! Text messages (KDE Connect's "SMS"): the phone's conversations, read here, and messages sent from here through the
//! phone. Android's KDE Connect app does this (with its SMS permission); iPhones can't. Hearth asks the phone for the
//! latest message of each conversation when it links, and for a whole conversation when you open it; nothing is
//! saved on this PC (the phone keeps them).

use crate::daemon::Daemon;
use crate::packet::Packet;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

const PER_THREAD: usize = 300;
const THREADS: usize = 200;

#[derive(Default)]
pub struct State {
    /// thread id → its messages, oldest first.
    pub threads: HashMap<i64, Vec<Value>>,
}

fn int(v: &Value) -> Option<i64> { v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)).or_else(|| v.as_str().and_then(|s| s.parse().ok())) }

/// One message from the phone, as Hearth keeps it.
pub fn message(v: &Value) -> Option<(i64, Value)> {
    let thread = int(v.get("thread_id")?)?;
    let addresses: Vec<String> = v.get("addresses").and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(|x| x.get("address").and_then(|s| s.as_str()).map(String::from)).collect()).unwrap_or_default();
    Some((thread, json!({
        "id": v.get("_id").and_then(int).unwrap_or(0), "body": v.get("body").and_then(|b| b.as_str()).unwrap_or(""),
        "date": v.get("date").and_then(int).unwrap_or(0), "sent": v.get("type").and_then(int) == Some(2),
        "read": v.get("read").and_then(int).unwrap_or(1) != 0, "addresses": addresses, "thread": thread,
    })))
}

pub fn packet(d: &Arc<Daemon>, id: &str, p: &Packet) {
    let list = p.body.get("messages").and_then(|m| m.as_array()).cloned().unwrap_or_default();
    let mut new = 0;
    super::with(id, |s| {
        for v in &list {
            let Some((t, m)) = message(v) else { continue };
            let th = s.sms.threads.entry(t).or_default();
            if th.iter().any(|x| x["id"] == m["id"] && x["date"] == m["date"]) { continue; }
            th.push(m);
            new += 1;
            th.sort_by_key(|x| x["date"].as_i64().unwrap_or(0));
            if th.len() > PER_THREAD { let cut = th.len() - PER_THREAD; th.drain(..cut); }
        }
        if s.sms.threads.len() > THREADS {
            let mut by: Vec<(i64, i64)> = s.sms.threads.iter().map(|(k, v)| (*k, v.last().and_then(|m| m["date"].as_i64()).unwrap_or(0))).collect();
            by.sort_by_key(|x| std::cmp::Reverse(x.1));
            for (k, _) in by.into_iter().skip(THREADS) { s.sms.threads.remove(&k); }
        }
    });
    d.emit("sms", json!({"device": id, "new": new}));
}

/// The conversations, newest first: {thread, addresses, last, date, sent, unread}.
pub fn threads(id: &str) -> Vec<Value> {
    let mut out: Vec<Value> = super::with(id, |s| s.sms.threads.iter().filter_map(|(t, msgs)| {
        let last = msgs.last()?;
        Some(json!({"thread": t, "addresses": last["addresses"], "last": last["body"], "date": last["date"], "sent": last["sent"],
            "unread": msgs.iter().filter(|m| !m["read"].as_bool().unwrap_or(true) && !m["sent"].as_bool().unwrap_or(false)).count()}))
    }).collect());
    out.sort_by_key(|v| std::cmp::Reverse(v["date"].as_i64().unwrap_or(0)));
    // Names from the phone's contacts (looked up after, not inside, the state lock).
    for t in out.iter_mut() {
        let names: Vec<String> = t["addresses"].as_array().cloned().unwrap_or_default().iter().filter_map(|a| a.as_str())
            .map(|a| super::contacts::name_for(id, a).unwrap_or_else(|| a.to_string())).collect();
        t["names"] = json!(names);
    }
    out
}

pub fn messages(id: &str, thread: i64) -> Vec<Value> { super::with(id, |s| s.sms.threads.get(&thread).cloned().unwrap_or_default()) }

pub fn addresses_of(id: &str, thread: i64) -> Vec<String> {
    super::with(id, |s| s.sms.threads.get(&thread).and_then(|m| m.last()).and_then(|m| m["addresses"].as_array().cloned()).unwrap_or_default())
        .into_iter().filter_map(|a| a.as_str().map(String::from)).collect()
}

/// Asks for the newest message of every conversation.
pub fn request_threads(d: &Daemon, id: &str) { let _ = d.send_body(id, "kdeconnect.sms.request_conversations", json!({})); }

/// Asks for a conversation's messages (the last 50).
pub fn request_thread(d: &Daemon, id: &str, thread: i64) -> Result<(), String> {
    d.send_body(id, "kdeconnect.sms.request_conversation", json!({"threadID": thread, "numberToRequest": 50}))
}

/// Sends a text message through the phone (version 2 of the packet, with the old fields for older apps).
pub fn send(d: &Daemon, id: &str, to: Vec<String>, text: &str) -> Result<Value, String> {
    let to: Vec<String> = to.into_iter().map(|a| a.trim().to_string()).filter(|a| !a.is_empty() && a.len() < 64).collect();
    if to.is_empty() { return Err("Who to? Type a phone number".into()); }
    if text.trim().is_empty() { return Err("Type a message first".into()); }
    let addresses: Vec<Value> = to.iter().map(|a| json!({"address": a})).collect();
    d.send_body(id, "kdeconnect.sms.request", json!({"version": 2, "addresses": addresses, "messageBody": text, "sendSms": true, "phoneNumber": to[0]}))?;
    d.emit("sms-sent", json!({"device": id, "to": to}));
    Ok(json!({"to": to}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_phone_messages() {
        let (t, m) = message(&json!({"event": 1, "body": "Hi", "addresses": [{"address": "+15550100"}], "date": 1700000000000i64, "type": 1, "read": 0, "thread_id": 7, "_id": 31})).unwrap();
        assert_eq!(t, 7);
        assert_eq!(m["body"], "Hi");
        assert_eq!(m["read"], false);
        assert_eq!(m["sent"], false);
        assert_eq!(m["addresses"][0], "+15550100");
        assert!(message(&json!({"body": "no thread"})).is_none());
    }
}
