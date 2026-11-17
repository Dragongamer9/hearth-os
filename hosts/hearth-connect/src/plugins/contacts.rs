//! The phone's contacts (KDE Connect's "Contacts"): Hearth asks which contacts the phone has (their ids and when each
//! last changed), then fetches the new and changed ones as vCards. They give names to the numbers in Messages and in
//! "… is calling", and Settings › Phone lists them. Kept only while the service runs (the phone has them).

use crate::daemon::Daemon;
use crate::packet::Packet;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default)]
pub struct State {
    /// uid → (changed, {name, numbers})
    pub people: HashMap<String, (i64, Value)>,
    /// Asked for, with the time each last changed (the vCards come without it).
    pub pending: HashMap<String, i64>,
}

/// Name and phone numbers from a vCard (FN, else N; every TEL).
pub fn parse_vcard(text: &str) -> Value {
    // Folded lines (a line starting with a space continues the one before).
    let mut lines: Vec<String> = vec![];
    for l in text.lines() {
        let l = l.trim_end_matches('\r');
        if (l.starts_with(' ') || l.starts_with('\t')) && !lines.is_empty() { let last = lines.len() - 1; lines[last].push_str(&l[1..]); } else { lines.push(l.to_string()); }
    }
    let (mut name, mut n, mut numbers) = (String::new(), String::new(), vec![]);
    for l in &lines {
        let Some((k, v)) = l.split_once(':') else { continue };
        let key = k.split(';').next().unwrap_or("").to_ascii_uppercase();
        let key = key.rsplit('.').next().unwrap_or("").to_string();
        match key.as_str() {
            "FN" => name = v.trim().to_string(),
            "N" => n = v.split(';').filter(|x| !x.is_empty()).take(2).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join(" "),
            "TEL" => { let t = v.trim(); if !t.is_empty() { numbers.push(t.to_string()); } }
            _ => {}
        }
    }
    if name.is_empty() { name = n; }
    json!({"name": name, "numbers": numbers})
}

/// Digits only, the last 9 (so +1 555 0100 and 5550100 match).
pub fn number_key(n: &str) -> String {
    let d: String = n.chars().filter(|c| c.is_ascii_digit()).collect();
    d.chars().rev().take(9).collect::<Vec<_>>().into_iter().rev().collect()
}

pub fn request(d: &Daemon, id: &str) { let _ = d.send_body(id, "kdeconnect.contacts.request_all_uids_timestamps", json!({})); }

pub fn packet(d: &Arc<Daemon>, id: &str, p: &Packet) {
    let uids: Vec<String> = p.body.get("uids").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    match p.kind.as_str() {
        "kdeconnect.contacts.response_uids_timestamps" => {
            let want: Vec<String> = super::with(id, |s| {
                s.contacts.people.retain(|k, _| uids.contains(k));
                let mut want = vec![];
                for u in &uids {
                    let ts = p.int(u).unwrap_or(0);
                    if s.contacts.people.get(u).map(|(t, _)| *t != ts).unwrap_or(true) { s.contacts.pending.insert(u.clone(), ts); want.push(u.clone()); }
                }
                want
            });
            if !want.is_empty() { let _ = d.send_body(id, "kdeconnect.contacts.request_vcards_by_uid", json!({"uids": want})); }
        }
        "kdeconnect.contacts.response_vcards" => {
            super::with(id, |s| for u in &uids {
                if let Some(card) = p.str(u) { let ts = s.contacts.pending.remove(u).unwrap_or(1); s.contacts.people.insert(u.clone(), (ts, parse_vcard(card))); }
            });
            d.emit("contacts", json!({"device": id, "count": uids.len()}));
            d.changed();
        }
        _ => {}
    }
}

/// The contact's name for a number, if the phone has one.
pub fn name_for(id: &str, number: &str) -> Option<String> {
    let k = number_key(number);
    if k.len() < 4 { return None; }
    super::with(id, |s| s.contacts.people.values().find(|(_, c)| c["numbers"].as_array().map(|a| a.iter().any(|n| n.as_str().map(number_key).as_deref() == Some(k.as_str()))).unwrap_or(false))
        .and_then(|(_, c)| c["name"].as_str().filter(|n| !n.is_empty()).map(String::from)))
}

/// Everyone, by name.
pub fn list(id: &str) -> Vec<Value> {
    let mut v: Vec<Value> = super::with(id, |s| s.contacts.people.values().map(|(_, c)| c.clone()).collect());
    v.sort_by_key(|c| c["name"].as_str().unwrap_or("").to_lowercase());
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vcards() {
        let c = parse_vcard("BEGIN:VCARD\r\nVERSION:2.1\r\nN:Lovelace;Ada;;;\r\nTEL;CELL:+44 20 7946 0018\r\nTEL;HOME:020 7946 0019\r\nEND:VCARD\r\n");
        assert_eq!(c["name"], "Ada Lovelace");
        assert_eq!(c["numbers"][1], "020 7946 0019");
        let c = parse_vcard("BEGIN:VCARD\nFN:Grace Hopper\nitem1.TEL:555-0100\nEND:VCARD");
        assert_eq!(c["name"], "Grace Hopper");
        assert_eq!(c["numbers"][0], "555-0100");
        assert_eq!(number_key("+1 (555) 010-0123"), "550100123");
    }
}
